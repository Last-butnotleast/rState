use std::{cell::RefCell, rc::Rc};

use rstate::StoreError;

use super::{Event, selected, store};

#[test]
fn combined_listeners_reconcile_nested_independent_store_dispatch() {
    let left_store = store();
    let right_store = store();
    let left = selected(&left_store);
    let right = selected(&right_store);
    let combined = left.combine(
        &right,
        |left, right| left.saturating_add(*right),
        |a, b| a == b,
    );
    let trace = Rc::new(RefCell::new(Vec::new()));
    let first_trace = trace.clone();
    let nested = right_store.downgrade();
    let first = combined.subscribe(move |value| {
        first_trace.borrow_mut().push(format!("a:{value}"));
        if *value == 1 {
            nested.upgrade().unwrap().send(Event::Add(2))?;
            first_trace.borrow_mut().push("a-return".to_owned());
        }
        Ok(())
    });
    let second_trace = trace.clone();
    let second = combined.subscribe(move |value| {
        second_trace.borrow_mut().push(format!("b:{value}"));
        Ok(())
    });

    left_store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), ["a:1", "b:3", "a-return"]);
    assert_eq!(*combined.get(), 3);
    first.unsubscribe();
    second.unsubscribe();
}

#[test]
fn combined_callback_errors_drain_remaining_observers_and_keep_committed_state() {
    let left_store = store();
    let right_store = store();
    let combined = selected(&left_store).combine(
        &selected(&right_store),
        |left, right| left.saturating_add(*right),
        |a, b| a == b,
    );
    let trace = Rc::new(RefCell::new(Vec::new()));
    let first_trace = trace.clone();
    let bad = combined.subscribe(move |value| {
        first_trace.borrow_mut().push(format!("bad:{value}"));
        Err(StoreError::new("combined failure"))
    });
    let second_trace = trace.clone();
    let good = combined.subscribe(move |value| {
        second_trace.borrow_mut().push(format!("good:{value}"));
        Ok(())
    });
    let direct_trace = trace.clone();
    let direct = left_store.subscribe(move |snapshot| {
        direct_trace
            .borrow_mut()
            .push(format!("direct:{}", snapshot.context.count));
        Ok(())
    });

    assert_eq!(
        left_store.send(Event::Add(1)).unwrap_err().to_string(),
        "combined failure"
    );
    assert_eq!(*trace.borrow(), ["bad:1", "good:1", "direct:1"]);
    assert_eq!(left_store.get_snapshot().context.count, 1);
    assert_eq!(*combined.get(), 1);
    bad.unsubscribe();
    good.unsubscribe();
    direct.unsubscribe();
}
