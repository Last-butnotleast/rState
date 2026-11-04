use std::{cell::RefCell, rc::Rc};

use rstate::{EffectScope, Store, StoreConfig, StoreError, Update};

enum Event {
    Start,
    Add(i32),
}

#[test]
fn synchronous_effects_read_the_dispatch_snapshot_then_retained_scopes_read_live_state() {
    let retained = Rc::new(RefCell::new(None::<EffectScope<i32, Event>>));
    let retained_scope = Rc::clone(&retained);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let effect_trace = Rc::clone(&trace);
    let store = Store::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event: &Event, enqueue| match event {
            Event::Start => {
                let retained = Rc::clone(&retained_scope);
                let first_trace = Rc::clone(&effect_trace);
                enqueue.effect(move |scope| {
                    *retained.borrow_mut() = Some(scope.clone());
                    first_trace.borrow_mut().push(*scope.get_snapshot().context);
                    scope.send(Event::Add(10))?;
                    first_trace.borrow_mut().push(*scope.get_snapshot().context);
                    Ok(())
                });
                let second_trace = Rc::clone(&effect_trace);
                enqueue.effect(move |scope| {
                    second_trace
                        .borrow_mut()
                        .push(*scope.get_snapshot().context);
                    Ok(())
                });
                Ok(Update::replace(**context + 1))
            }
            Event::Add(by) => Ok(Update::replace(**context + by)),
        },
    ));

    store.send(Event::Start).unwrap();

    assert_eq!(*trace.borrow(), [1, 1, 1]);
    assert_eq!(*store.get_snapshot().context, 11);
    let scope = retained.borrow().as_ref().unwrap().clone();
    assert_eq!(*scope.get_snapshot().context, 11);

    scope.send(Event::Add(2)).unwrap();
    assert_eq!(*store.get_snapshot().context, 13);
    assert_eq!(*scope.get_snapshot().context, 13);
}

#[test]
fn failed_effect_loops_keep_retained_scopes_pinned_and_skip_later_effects() {
    let retained = Rc::new(RefCell::new(None::<EffectScope<i32, Event>>));
    let retained_scope = Rc::clone(&retained);
    let later_runs = Rc::new(RefCell::new(0));
    let effect_runs = Rc::clone(&later_runs);
    let store = Store::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event: &Event, enqueue| match event {
            Event::Start => {
                let retained = Rc::clone(&retained_scope);
                enqueue.effect(move |scope| {
                    *retained.borrow_mut() = Some(scope);
                    Err(StoreError::new("effect failed"))
                });
                let runs = Rc::clone(&effect_runs);
                enqueue.effect(move |_| {
                    *runs.borrow_mut() += 1;
                    Ok(())
                });
                Ok(Update::replace(**context + 1))
            }
            Event::Add(by) => Ok(Update::replace(**context + by)),
        },
    ));

    let error = store.send(Event::Start).unwrap_err();

    assert_eq!(error.to_string(), "effect failed");
    assert_eq!(*store.get_snapshot().context, 1);
    assert_eq!(*later_runs.borrow(), 0);
    store.send(Event::Add(10)).unwrap();
    assert_eq!(*store.get_snapshot().context, 11);
    assert_eq!(
        *retained.borrow().as_ref().unwrap().get_snapshot().context,
        1
    );
}

#[test]
fn retained_scope_debug_reports_completion_without_replaying_effects() {
    let retained = Rc::new(RefCell::new(None::<EffectScope<i32, Event>>));
    let retained_scope = retained.clone();
    let calls = Rc::new(RefCell::new(0));
    let effect_calls = calls.clone();
    let store =
        Store::<i32, Event>::new(StoreConfig::new(0, move |context: &Rc<i32>, _, enqueue| {
            let retained = retained_scope.clone();
            let calls = effect_calls.clone();
            enqueue.effect(move |scope| {
                assert!(format!("{scope:?}").contains("completed: false"));
                *calls.borrow_mut() += 1;
                retained.replace(Some(scope));
                Ok(())
            });
            Ok(Update::replace(**context + 1))
        }));

    store.send(Event::Start).unwrap();
    let scope = retained.borrow().as_ref().unwrap().clone();

    assert!(format!("{scope:#?}").contains("completed: true"));
    assert_eq!(*calls.borrow(), 1);
}
