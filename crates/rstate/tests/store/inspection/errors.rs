use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, StoreError};

use super::support::{Event, describe, store};

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
fn a_failed_initial_report_leaves_the_registered_inspector_active() {
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
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), ["init:0", "inspect:add:1:1"]);
}
