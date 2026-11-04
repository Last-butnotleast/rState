use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, StoreError};

use super::support::{Event, describe, store};

struct OnDrop {
    callback: Box<dyn Fn()>,
    _marker: Rc<()>,
}

impl Drop for OnDrop {
    fn drop(&mut self) {
        (self.callback)();
    }
}

#[test]
fn inspector_errors_stop_later_inspectors_and_effects_after_commit() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let state_trace = trace.clone();
    let state = store.subscribe(move |snapshot| {
        state_trace
            .borrow_mut()
            .push(format!("state:{}", snapshot.context.count));
        Ok(())
    });
    let first_trace = trace.clone();
    let first = store
        .inspect(move |inspection| {
            if matches!(inspection, Inspection::Transition { .. }) {
                first_trace.borrow_mut().push("bad".into());
                return Err(StoreError::new("inspection failed"));
            }
            Ok(())
        })
        .unwrap();
    let second_trace = trace.clone();
    let second = store
        .inspect(move |inspection| {
            second_trace.borrow_mut().push(describe(&inspection));
            Ok(())
        })
        .unwrap();
    trace.borrow_mut().clear();

    let error = store.send(Event::Start).unwrap_err();

    assert_eq!(error.to_string(), "inspection failed");
    assert_eq!(*trace.borrow(), ["state:2", "bad"]);
    assert_eq!(store.get_snapshot().context.count, 2);

    first.unsubscribe();
    store.send(Event::Start).unwrap();
    assert_eq!(
        *trace.borrow(),
        ["state:2", "bad", "state:4", "inspect:start:4", "effect:4"]
    );
    state.unsubscribe();
    second.unsubscribe();
}

#[test]
fn subscriber_errors_skip_inspection_and_effects_while_effect_errors_keep_inspection() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let first_trace = trace.clone();
    let bad = store.subscribe(move |_| {
        first_trace.borrow_mut().push("bad state".into());
        Err(StoreError::new("state failed"))
    });
    let second_trace = trace.clone();
    let good = store.subscribe(move |_| {
        second_trace.borrow_mut().push("good state".into());
        Ok(())
    });
    let observed = trace.clone();
    let inspector = store
        .inspect(move |inspection| {
            observed.borrow_mut().push(describe(&inspection));
            Ok(())
        })
        .unwrap();
    trace.borrow_mut().clear();

    assert_eq!(
        store.send(Event::Start).unwrap_err().to_string(),
        "state failed"
    );
    assert_eq!(*trace.borrow(), ["bad state", "good state"]);
    assert_eq!(store.get_snapshot().context.count, 2);
    bad.unsubscribe();
    good.unsubscribe();

    assert_eq!(
        store.send(Event::EffectFail).unwrap_err().to_string(),
        "effect failed"
    );
    assert_eq!(
        *trace.borrow(),
        ["bad state", "good state", "inspect:effectFail:3"]
    );
    assert_eq!(store.get_snapshot().context.count, 3);
    inspector.unsubscribe();
}

#[test]
fn a_failed_initial_report_unregisters_its_inspector() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let observed = trace.clone();
    let error = store
        .inspect(move |inspection| {
            observed.borrow_mut().push(describe(&inspection));
            if matches!(inspection, Inspection::Initial { .. }) {
                Err(StoreError::new("initial inspection failed"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();

    assert_eq!(error.to_string(), "initial inspection failed");
    assert_eq!(error.dispatch_status(), None);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), ["init:0"]);
}

#[test]
fn a_failed_initial_report_preserves_nested_dispatch_and_other_registrations() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let good_trace = trace.clone();
    let good = store
        .inspect(move |record| {
            good_trace
                .borrow_mut()
                .push(format!("good:{}", describe(&record)));
            Ok(())
        })
        .unwrap();
    trace.borrow_mut().clear();
    let bad_trace = trace.clone();
    let weak = store.downgrade();

    let error = store
        .inspect(move |record| {
            bad_trace
                .borrow_mut()
                .push(format!("bad:{}", describe(&record)));
            if matches!(record, Inspection::Initial { .. }) {
                let store = weak.upgrade().unwrap();
                store.send(Event::Add(1))?;
                let new_trace = bad_trace.clone();
                drop(store.inspect(move |record| {
                    new_trace
                        .borrow_mut()
                        .push(format!("new:{}", describe(&record)));
                    Ok(())
                })?);
                Err(StoreError::new("initial inspection failed"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();

    assert_eq!(error.dispatch_status(), None);
    assert_eq!(store.get_snapshot().context.count, 1);
    store.send(Event::Add(2)).unwrap();
    assert_eq!(
        *trace.borrow(),
        [
            "bad:init:0",
            "good:inspect:add:1:1",
            "bad:inspect:add:1:1",
            "new:init:1",
            "good:inspect:add:2:3",
            "new:inspect:add:2:3",
        ]
    );
    good.unsubscribe();
}

#[test]
fn failed_registration_releases_captures_before_return_and_allows_drop_reentry() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let marker = Rc::new(());
    let retained = Rc::downgrade(&marker);
    let weak = store.downgrade();
    let dropped_trace = trace.clone();
    let resource = OnDrop {
        callback: Box::new(move || {
            let store = weak.upgrade().unwrap();
            let guard = store.inspect(|_| Ok(())).unwrap().scoped();
            store.send(Event::Add(1)).unwrap();
            dropped_trace.borrow_mut().push("released".into());
            drop(guard);
        }),
        _marker: marker,
    };

    let error = store
        .inspect(move |_| {
            let _retained = &resource;
            Err(StoreError::new("initial inspection failed"))
        })
        .unwrap_err();

    assert_eq!(error.to_string(), "initial inspection failed");
    assert_eq!(error.dispatch_status(), None);
    assert!(retained.upgrade().is_none());
    assert_eq!(store.get_snapshot().context.count, 1);
    assert_eq!(*trace.borrow(), ["released"]);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
}
