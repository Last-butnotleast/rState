use std::{cell::RefCell, error::Error, io, rc::Rc};

use rstate::{
    DispatchStatus, Inspection, Schema, SchemaCheck, SchemaMetadata, Schemas, Store, StoreConfig,
    StoreError, StoreFactory, Update, ValidationOptions, WeakStore,
};

enum Event {
    Add,
    Keep,
    Fail,
    EffectFail,
    Emit,
}

#[derive(Clone, Copy, Debug)]
enum FailurePath {
    Subscriber,
    Inspector,
    Effect,
    Emission,
}

fn application_failure() -> StoreError {
    StoreError::from_error(io::Error::other("application failed"))
}

fn store() -> Store<i32, Event, &'static str> {
    Store::new(StoreConfig::new(0, |context: &Rc<i32>, event, enqueue| {
        match event {
            Event::Fail => return Err(application_failure()),
            Event::Keep => return Ok(Update::Keep),
            Event::EffectFail => enqueue.effect(|_| Err(application_failure())),
            Event::Emit => enqueue.emit("value"),
            Event::Add => {}
        }
        Ok(Update::replace(context.saturating_add(1)))
    }))
}

fn counting_store() -> Store<i32, i32> {
    Store::new(StoreConfig::new(0, |context: &Rc<i32>, by: &i32, _| {
        Ok(Update::replace(context.saturating_add(*by)))
    }))
}

fn committed_failure_store() -> Store<i32, i32> {
    let store = counting_store();
    drop(store.subscribe(|_| Err(application_failure())));
    store
}

#[test]
fn reducer_errors_report_no_commit_and_preserve_the_original_source() {
    let store = store();
    let original = store.get_snapshot();

    let error = store.send(Event::Fail).unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
    match error.dispatch_status().unwrap() {
        DispatchStatus::NotCommitted => assert!(Rc::ptr_eq(&original, &store.get_snapshot())),
        DispatchStatus::Committed => panic!("reducer failure must not publish its transition"),
    }
    assert_eq!(error.to_string(), "application failed");
    assert!(error.source().unwrap().is::<io::Error>());
}

#[test]
fn postcommit_callback_errors_keep_the_original_cause_and_committed_state() {
    for path in [
        FailurePath::Subscriber,
        FailurePath::Inspector,
        FailurePath::Effect,
        FailurePath::Emission,
    ] {
        let store = store();
        let event = match path {
            FailurePath::Subscriber => {
                drop(store.subscribe(|_| Err(application_failure())));
                Event::Add
            }
            FailurePath::Inspector => {
                drop(
                    store
                        .inspect(|record| match record {
                            Inspection::Transition { .. } => Err(application_failure()),
                            _ => Ok(()),
                        })
                        .unwrap(),
                );
                Event::Add
            }
            FailurePath::Effect => Event::EffectFail,
            FailurePath::Emission => {
                drop(store.on_any(|_| Err(application_failure())));
                Event::Emit
            }
        };

        let error = store.send(event).unwrap_err();

        assert_eq!(
            error.dispatch_status(),
            Some(DispatchStatus::Committed),
            "{path:?}"
        );
        assert_eq!(*store.get_snapshot().context, 1, "{path:?}");
        assert_eq!(error.to_string(), "application failed");
        assert!(error.source().unwrap().is::<io::Error>());
        assert!(error.source().unwrap().source().is_none());
    }
}

#[test]
fn an_inspector_failure_after_a_noop_is_committed_without_snapshot_replacement() {
    let store = store();
    let original = store.get_snapshot();
    drop(
        store
            .inspect(|record| match record {
                Inspection::Transition { .. } => Err(application_failure()),
                _ => Ok(()),
            })
            .unwrap(),
    );

    let error = store.send(Event::Keep).unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::Committed));
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
}

#[test]
fn nested_commits_do_not_make_a_failed_outer_evaluation_committed() {
    let inner = committed_failure_store();
    let nested = inner.clone();
    let outer: Store<i32, ()> = Store::new(StoreConfig::new(0, move |_, (), _| {
        nested.send(1)?;
        Ok(Update::replace(9))
    }));
    let original = outer.get_snapshot();

    let error = outer.send(()).unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
    assert!(Rc::ptr_eq(&original, &outer.get_snapshot()));
    assert_eq!(*inner.get_snapshot().context, 1);
    assert!(error.source().unwrap().is::<io::Error>());
}

#[test]
fn a_reentrant_same_store_commit_does_not_change_the_failed_outer_status() {
    enum ReentrantEvent {
        Outer,
        Inner,
    }

    let handle: Rc<RefCell<Option<WeakStore<i32, ReentrantEvent>>>> = Rc::new(RefCell::new(None));
    let nested = handle.clone();
    let store = Store::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event, _| match event {
            ReentrantEvent::Outer => {
                let store = nested.borrow().as_ref().unwrap().upgrade().unwrap();
                store.send(ReentrantEvent::Inner)?;
                Err(application_failure())
            }
            ReentrantEvent::Inner => Ok(Update::replace(context.saturating_add(1))),
        },
    ));
    handle.replace(Some(store.downgrade()));

    let error = store.send(ReentrantEvent::Outer).unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
    assert_eq!(*store.get_snapshot().context, 1);
}

#[test]
fn a_nested_precommit_error_does_not_hide_an_outer_commit() {
    let inner: Store<i32, ()> =
        Store::new(StoreConfig::new(0, |_, (), _| Err(application_failure())));
    let nested = inner.clone();
    let outer = counting_store();
    let own = outer.subscribe(move |_| nested.send(()));

    let error = outer.send(1).unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::Committed));
    assert_eq!(*outer.get_snapshot().context, 1);
    assert_eq!(*inner.get_snapshot().context, 0);
    own.unsubscribe();
}

#[test]
fn dry_run_queries_do_not_inherit_a_nested_dispatch_status() {
    let nested = committed_failure_store();
    let store: Store<i32, ()> = Store::new(StoreConfig::new(0, move |_, (), _| {
        nested.send(1)?;
        Ok(Update::Keep)
    }));
    let original = store.get_snapshot();

    let transition = store.transition(original.clone(), ()).unwrap_err();
    let availability = store.can(()).unwrap_err();

    assert_eq!(transition.dispatch_status(), None);
    assert_eq!(availability.dispatch_status(), None);
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
}

#[test]
fn schemas_and_factory_initializers_do_not_inherit_nested_dispatch_status() {
    let nested = committed_failure_store();
    let schema = Schema::new(SchemaMetadata::new("external check"), move |(): &()| {
        nested.send(1)?;
        Ok(SchemaCheck::Valid)
    });

    let error = schema.check(&()).unwrap_err();

    assert_eq!(error.dispatch_status(), None);
    assert!(error.source().unwrap().is::<io::Error>());

    let nested = committed_failure_store();
    let factory: StoreFactory<(), i32, ()> = StoreFactory::new(
        move |()| {
            nested.send(1)?;
            Ok(0)
        },
        |_, (), _| Ok(Update::Keep),
    );

    let error = factory.create(()).unwrap_err();

    assert_eq!(error.dispatch_status(), None);
    assert!(error.source().unwrap().is::<io::Error>());
}

#[test]
fn initial_inspection_does_not_inherit_a_nested_dispatch_status() {
    let nested = committed_failure_store();
    let store = store();

    let error = store.inspect(move |_| nested.send(1)).unwrap_err();

    assert_eq!(error.dispatch_status(), None);
    assert!(error.source().unwrap().is::<io::Error>());
    store.send(Event::Add).unwrap();
    assert_eq!(*store.get_snapshot().context, 1);
}

#[test]
fn validation_provider_failure_reports_no_commit_even_when_its_provider_dispatched() {
    let nested = committed_failure_store();
    let events = Schema::new(SchemaMetadata::new("events"), move |event: &bool| {
        if *event {
            nested.send(1)?;
        }
        Ok(SchemaCheck::Valid)
    });
    let store: Store<i32, bool> = Store::new_validated(
        StoreConfig::new(0, |_, _, _| Ok(Update::replace(1)))
            .with_schemas(Schemas::default().with_events(events)),
        ValidationOptions::default(),
    )
    .unwrap();
    let original = store.get_snapshot();

    let error = store.send(true).unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
}

#[test]
fn reset_callback_errors_report_no_commit_even_after_a_nested_commit() {
    let nested = committed_failure_store();
    let store = store();
    let original = store.get_snapshot();

    let error = store
        .reset_with(move |_, _| {
            nested.send(1)?;
            Ok(Rc::new(9))
        })
        .unwrap_err();

    assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
}

#[test]
fn reset_and_empty_history_control_callback_failures_are_committed() {
    let store = store();
    drop(
        store
            .inspect(|record| match record {
                Inspection::Control { .. } => Err(application_failure()),
                _ => Ok(()),
            })
            .unwrap(),
    );

    for result in [store.reset(), store.undo(), store.redo()] {
        let error = result.unwrap_err();
        assert_eq!(error.dispatch_status(), Some(DispatchStatus::Committed));
        assert!(error.source().unwrap().is::<io::Error>());
    }
    assert_eq!(*store.get_snapshot().context, 0);
}
