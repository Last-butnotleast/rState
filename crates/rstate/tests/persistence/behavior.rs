use std::{cell::RefCell, error::Error, rc::Rc};

use rstate::{
    Inspection, Persistence, PersistenceError, Store, StoreConfig, StoreFactory, StoreResult,
    Update,
};

use super::support::{Codec, Context, Event, Storage, config, persistence};

#[test]
fn hydration_keeps_factory_defaults_separate_from_initial_history_and_saves_controls() {
    let storage = Storage::default();
    storage.seed("1:10");
    let effects = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new_persisted(
        config(0, effects.clone()).with_history(3),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    assert_eq!(store.get_initial_snapshot().context.value, 10);
    assert!(!store.can_undo());
    assert_eq!(*storage.calls.borrow(), ["read:state"]);

    store.send(Event::Add(2)).unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:12"));
    store.undo().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:10"));
    store.redo().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:12"));
    store.reset().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:0"));
    store.undo().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:12"));
    assert_eq!(store.get_initial_snapshot().context.value, 10);
    assert_eq!(*effects.borrow(), ["effect"]);

    let before = store.get_snapshot();
    storage.calls.borrow_mut().clear();
    store.clear_storage().unwrap();
    assert_eq!(storage.saved(), None);
    assert!(Rc::ptr_eq(&before, &store.get_snapshot()));
    assert!(store.can_undo());
    store.save().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:12"));
}

#[test]
fn autosave_runs_after_observers_and_inspection_before_effects_and_dry_runs_are_silent() {
    let storage = Storage::default();
    let trace = storage.calls.clone();
    let store = Store::new_persisted(
        config(0, trace.clone()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let observed = trace.clone();
    let _subscription = store.subscribe(move |_| {
        observed.borrow_mut().push("observe".into());
        Ok(())
    });
    let inspected = trace.clone();
    let _inspection = store
        .inspect(move |record| {
            if matches!(record, Inspection::Transition { .. }) {
                inspected.borrow_mut().push("inspect".into());
            }
            Ok(())
        })
        .unwrap();
    trace.borrow_mut().clear();
    assert!(store.can(Event::Add(2)).unwrap());
    store
        .transition(store.get_snapshot(), Event::Add(2))
        .unwrap();
    assert!(trace.borrow().is_empty());

    store.send(Event::Add(2)).unwrap();
    assert_eq!(
        *trace.borrow(),
        ["observe", "inspect", "write:1:2", "effect"]
    );
    trace.borrow_mut().clear();
    store.send(Event::Keep).unwrap();
    assert_eq!(*trace.borrow(), ["inspect", "write:1:2"]);
    trace.borrow_mut().clear();
    assert!(store.send(Event::Fail).is_err());
    assert!(trace.borrow().is_empty());
}

#[test]
fn automatic_errors_preserve_committed_actions_and_sources_until_successful_retry() {
    let storage = Storage::default();
    let codec = Codec::default();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let error_log = errors.clone();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new_persisted(
        config(0, effects.clone()),
        persistence(&storage, &codec).on_error(move |error| {
            error_log.borrow_mut().push(error.to_string());
        }),
    )
    .unwrap();
    storage.fail_write.set(true);
    store.send(Event::Add(3)).unwrap();
    assert_eq!(store.get_snapshot().context.value, 3);
    assert_eq!(*effects.borrow(), ["effect"]);
    assert_eq!(errors.borrow().len(), 1);
    let failure = store.persistence_error().unwrap();
    let application = failure.source().unwrap().source().unwrap();
    assert!(application.downcast_ref::<std::io::Error>().is_some());
    assert!(store.save().is_err());
    assert_eq!(store.get_snapshot().context.value, 3);

    storage.fail_write.set(false);
    store.save().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:3"));
    assert!(store.persistence_error().is_none());
    codec.fail_encode.set(true);
    store.send(Event::Add(1)).unwrap();
    assert!(matches!(
        store.persistence_error().as_deref(),
        Some(PersistenceError::Operation { .. })
    ));
    assert_eq!(storage.saved().as_deref(), Some("1:3"));
}

#[test]
fn load_corruption_versions_and_storage_errors_never_overwrite_saved_data() {
    for data in ["invalid", "2:8"] {
        let storage = Storage::default();
        storage.seed(data);
        let error = Store::new_persisted(
            config(0, Rc::new(RefCell::new(Vec::new()))),
            persistence(&storage, &Codec::default()),
        )
        .unwrap_err();
        assert!(error.source().is_some());
        assert_eq!(storage.saved().as_deref(), Some(data));
        assert_eq!(*storage.calls.borrow(), ["read:state"]);
    }
    let storage = Storage::default();
    storage.fail_read.set(true);
    assert!(
        Store::new_persisted(
            config(0, Rc::new(RefCell::new(Vec::new()))),
            persistence(&storage, &Codec::default()),
        )
        .is_err()
    );
}

#[test]
fn persistent_factories_and_keys_remain_independent_without_context_clone_or_serde() {
    let storage = Storage::default();
    let factory: StoreFactory<i64, Context, ()> = StoreFactory::new(
        |value| -> StoreResult<Context> {
            Ok(Context {
                value,
                local: Rc::default(),
            })
        },
        |_, &(), _| Ok(Update::Keep),
    );
    let first = factory
        .create_persisted(4, persistence(&storage, &Codec::default()))
        .unwrap();
    first.save().unwrap();
    let second = factory
        .create_persisted(9, persistence(&storage, &Codec::default()))
        .unwrap();
    assert_eq!(second.get_snapshot().context.value, 4);
    second.reset().unwrap();
    assert_eq!(second.get_snapshot().context.value, 9);
    assert_eq!(first.get_snapshot().context.value, 4);
    let separate = factory
        .create_persisted(
            6,
            Persistence::new("other", 1, storage.clone(), Codec::default()),
        )
        .unwrap();
    separate.save().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:9"));
    assert_eq!(
        storage.values.borrow().get("other").map(Vec::as_slice),
        Some(b"1:6".as_slice())
    );
    let ordinary: Store<(), ()> = Store::new(StoreConfig::new((), |_, &(), _| Ok(Update::Keep)));
    assert!(ordinary.save().is_err());
    assert!(ordinary.clear_storage().is_err());
    assert!(ordinary.persistence_error().is_none());
}

#[test]
fn subscriber_and_inspector_errors_prevent_automatic_writes_without_rolling_back_state() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let subscription = store.subscribe(|_| Err(rstate::StoreError::new("observer failed")));
    assert!(store.send(Event::Add(1)).is_err());
    assert_eq!(store.get_snapshot().context.value, 1);
    assert_eq!(storage.saved(), None);
    subscription.unsubscribe();
    let inspection = store
        .inspect(|record| match record {
            Inspection::Transition { .. } => Err(rstate::StoreError::new("inspection failed")),
            _ => Ok(()),
        })
        .unwrap();
    assert!(store.send(Event::Add(1)).is_err());
    assert_eq!(store.get_snapshot().context.value, 2);
    assert_eq!(storage.saved(), None);
    inspection.unsubscribe();
    store.save().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:2"));
}
