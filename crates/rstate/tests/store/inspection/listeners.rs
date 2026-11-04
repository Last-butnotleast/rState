use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, Subscription};

use super::support::{Event, describe, store};

#[test]
fn inspectors_added_during_notification_receive_init_and_the_current_outer_event() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let removed: Rc<RefCell<Option<Subscription>>> = Rc::new(RefCell::new(None));
    let added: Rc<RefCell<Option<Subscription>>> = Rc::new(RefCell::new(None));
    let first_trace = trace.clone();
    let remove_second = removed.clone();
    let added_subscription = added.clone();
    let nested = store.clone();
    let first = store
        .inspect(move |inspection| {
            if matches!(
                inspection,
                Inspection::Transition {
                    event: Event::Add(1),
                    ..
                }
            ) {
                first_trace.borrow_mut().push("first".into());
                let second = remove_second.borrow_mut().take();
                if let Some(second) = second {
                    second.unsubscribe();
                }
                let third_trace = first_trace.clone();
                let third = nested.inspect(move |inspection| {
                    third_trace
                        .borrow_mut()
                        .push(format!("third:{}", describe(&inspection)));
                    Ok(())
                })?;
                added_subscription.replace(Some(third));
            }
            Ok(())
        })
        .unwrap();
    let second_trace = trace.clone();
    let second = store
        .inspect(move |inspection| {
            if matches!(inspection, Inspection::Transition { .. }) {
                second_trace.borrow_mut().push("second".into());
            }
            Ok(())
        })
        .unwrap();
    removed.replace(Some(second));

    store.send(Event::Add(1)).unwrap();

    assert_eq!(
        *trace.borrow(),
        ["first", "third:init:1", "third:inspect:add:1:1"]
    );
    first.unsubscribe();
    added.borrow_mut().take().unwrap().unsubscribe();
}

#[test]
fn initial_callbacks_are_registered_before_synchronous_nested_dispatch() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let observed = trace.clone();
    let nested = store.clone();
    let subscription = store
        .inspect(move |inspection| {
            observed.borrow_mut().push(describe(&inspection));
            if matches!(inspection, Inspection::Initial { .. }) {
                nested.send(Event::Add(1))?;
                observed.borrow_mut().push(format!(
                    "init-return:{}",
                    nested.get_snapshot().context.count
                ));
            }
            Ok(())
        })
        .unwrap();

    assert_eq!(
        *trace.borrow(),
        ["init:0", "inspect:add:1:1", "init-return:1"]
    );
    subscription.unsubscribe();
}
