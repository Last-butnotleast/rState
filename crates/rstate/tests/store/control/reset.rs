use std::{cell::RefCell, rc::Rc};

use rstate::{Store, StoreError, StoreFactory, Update};

use super::support::{Context, Event, Trace, config, describe};

#[test]
fn reset_restores_defaults_with_fresh_snapshots_and_no_reducer_effects() {
    let trace: Trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(3, &trace));
    let initial = store.get_initial_snapshot();
    let observed = trace.clone();
    let _subscription = store.subscribe(move |snapshot| {
        observed
            .borrow_mut()
            .push(format!("observe:{}", snapshot.context.count));
        Ok(())
    });
    let inspected = trace.clone();
    let _inspector = store
        .inspect(move |record| {
            inspected.borrow_mut().push(describe(record));
            Ok(())
        })
        .unwrap();
    store.send(Event::Add(4)).unwrap();
    trace.borrow_mut().clear();

    store.reset().unwrap();
    let first = store.get_snapshot();
    store.reset().unwrap();
    let second = store.get_snapshot();

    assert_eq!(first.context.count, 3);
    assert!(Rc::ptr_eq(&initial.context, &first.context));
    assert!(Rc::ptr_eq(&initial.context, &second.context));
    assert!(!Rc::ptr_eq(&initial, &first));
    assert!(!Rc::ptr_eq(&first, &second));
    assert!(Rc::ptr_eq(&initial, &store.get_initial_snapshot()));
    assert_eq!(
        *trace.borrow(),
        [
            "observe:3",
            "control:Reset:3",
            "observe:3",
            "control:Reset:3"
        ]
    );
}

#[test]
fn custom_reset_can_preserve_fields_and_failure_does_not_commit() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(2, &trace));
    store.send(Event::Add(5)).unwrap();
    let previous = store.get_snapshot();
    let error = store
        .reset_with(|_, _| Err(StoreError::new("reset failed")))
        .unwrap_err();
    assert_eq!(error.to_string(), "reset failed");
    assert!(Rc::ptr_eq(&previous, &store.get_snapshot()));

    store
        .reset_with(|initial, current| {
            Ok(Rc::new(Context {
                count: initial.count,
                label: current.label.clone(),
                local: current.local.clone(),
            }))
        })
        .unwrap();
    let current = store.get_snapshot();
    assert_eq!(current.context.count, 2);
    assert_eq!(current.context.label, "changed");
    assert!(Rc::ptr_eq(&previous.context.local, &current.context.local));
}

#[test]
fn reset_keeps_its_dispatch_snapshot_when_a_subscriber_sends() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(0, &trace));
    store.send(Event::Add(2)).unwrap();
    let weak = store.downgrade();
    let _subscription = store.subscribe(move |snapshot| {
        if snapshot.context.count == 0 {
            weak.upgrade().unwrap().send(Event::Add(10))?;
        }
        Ok(())
    });
    let inspected = trace.clone();
    let _inspector = store
        .inspect(move |record| {
            inspected.borrow_mut().push(describe(record));
            Ok(())
        })
        .unwrap();
    trace.borrow_mut().clear();

    store.reset().unwrap();

    assert_eq!(store.get_snapshot().context.count, 10);
    assert_eq!(*trace.borrow(), ["event:Add(10):10", "control:Reset:0"]);
}

#[test]
fn factory_defaults_are_captured_per_input_and_remain_shared_without_clone_bounds() {
    let factory: StoreFactory<i32, i32, i32> =
        StoreFactory::new(Ok, |current: &Rc<i32>, by: &i32, _| {
            Ok(Update::replace(current.saturating_add(*by)))
        });
    let left = factory.create(2).unwrap();
    let right = factory.create(7).unwrap();
    left.send(3).unwrap();
    right.send(3).unwrap();
    left.reset().unwrap();
    right.reset().unwrap();
    assert_eq!(*left.get_snapshot().context, 2);
    assert_eq!(*right.get_snapshot().context, 7);
}
