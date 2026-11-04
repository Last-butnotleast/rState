use std::{cell::RefCell, rc::Rc};

use rstate::Inspection;

use super::support::{Event, describe, store};

#[test]
fn inspection_follows_state_observers_and_precedes_macrostep_effects() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let direct_trace = trace.clone();
    let direct = store.subscribe(move |snapshot| {
        direct_trace
            .borrow_mut()
            .push(format!("state:{}", snapshot.context.count));
        Ok(())
    });
    let selected = store.select(|context| context.count, |left, right| left == right);
    let selected_trace = trace.clone();
    let selected_subscription = selected.subscribe(move |value| {
        selected_trace
            .borrow_mut()
            .push(format!("selected:{value}"));
        Ok(())
    });
    let inspection_trace = trace.clone();
    let inspector = store
        .inspect(move |inspection| {
            inspection_trace.borrow_mut().push(describe(&inspection));
            Ok(())
        })
        .unwrap();
    let emission_trace = trace.clone();
    let emissions = store.on_any(move |message| {
        emission_trace.borrow_mut().push(format!("emit:{message}"));
        Ok(())
    });
    trace.borrow_mut().clear();

    store.send(Event::Start).unwrap();

    assert_eq!(
        *trace.borrow(),
        [
            "state:2",
            "selected:2",
            "inspect:start:2",
            "effect:2",
            "emit:parent",
            "emit:child"
        ]
    );
    direct.unsubscribe();
    selected_subscription.unsubscribe();
    inspector.unsubscribe();
    emissions.unsubscribe();
}

#[test]
fn subscriber_nested_dispatch_is_inspected_before_the_outer_transition() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let nested_store = store.clone();
    let direct = store.subscribe(move |snapshot| {
        if snapshot.context.count == 1 {
            nested_store.send(Event::Add(10))?;
        }
        Ok(())
    });
    let observed = trace.clone();
    let current = store.clone();
    let inspector = store
        .inspect(move |inspection| {
            if let Inspection::Transition {
                event, snapshot, ..
            } = inspection
            {
                let by = match event {
                    Event::Add(by) => *by,
                    _ => return Ok(()),
                };
                observed.borrow_mut().push(format!(
                    "inspect:{by}:{}:{}",
                    snapshot.context.count,
                    current.get_snapshot().context.count
                ));
            }
            Ok(())
        })
        .unwrap();

    store.send(Event::Add(1)).unwrap();

    assert_eq!(*trace.borrow(), ["inspect:10:11:11", "inspect:1:1:11"]);
    direct.unsubscribe();
    inspector.unsubscribe();
}

#[test]
fn inspector_nested_dispatch_retains_each_outer_record_snapshot() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let first_trace = trace.clone();
    let nested = store.clone();
    let first = store
        .inspect(move |inspection| {
            if let Inspection::Transition {
                event, snapshot, ..
            } = inspection
            {
                first_trace
                    .borrow_mut()
                    .push(format!("a:{}", snapshot.context.count));
                if matches!(event, Event::Add(1)) {
                    nested.send(Event::Add(10))?;
                    first_trace
                        .borrow_mut()
                        .push(format!("a-return:{}", nested.get_snapshot().context.count));
                }
            }
            Ok(())
        })
        .unwrap();
    let second_trace = trace.clone();
    let current = store.clone();
    let second = store
        .inspect(move |inspection| {
            if let Inspection::Transition { snapshot, .. } = inspection {
                second_trace.borrow_mut().push(format!(
                    "b:{}:{}",
                    snapshot.context.count,
                    current.get_snapshot().context.count
                ));
            }
            Ok(())
        })
        .unwrap();

    store.send(Event::Add(1)).unwrap();

    assert_eq!(
        *trace.borrow(),
        ["a:1", "a:11", "b:11:11", "a-return:11", "b:1:11"]
    );
    first.unsubscribe();
    second.unsubscribe();
}
