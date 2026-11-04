use std::{cell::RefCell, rc::Rc};

use rstate::{Store, StoreConfig, StoreError, Update};

use crate::support::{Event, counter};

#[test]
fn nested_send_reconciles_remaining_listeners_before_returning() {
    let store = counter();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let first_trace = Rc::clone(&trace);
    let nested = store.clone();
    let _first = store.subscribe(move |snapshot| {
        first_trace
            .borrow_mut()
            .push(format!("a:{}", snapshot.context.count));
        if snapshot.context.count == 1 {
            nested.send(Event::Add(10))?;
            first_trace
                .borrow_mut()
                .push(format!("a-return:{}", nested.get_snapshot().context.count));
        }
        Ok(())
    });
    let second_trace = Rc::clone(&trace);
    let _second = store.subscribe(move |snapshot| {
        second_trace
            .borrow_mut()
            .push(format!("b:{}", snapshot.context.count));
        Ok(())
    });

    store.send(Event::Add(1)).unwrap();

    assert_eq!(*trace.borrow(), ["a:1", "b:11", "a-return:11"]);
    assert_eq!(store.get_snapshot().context.count, 11);
}

#[test]
fn cross_store_nested_send_drains_the_shared_queue_before_returning() {
    let left = counter();
    let right = counter();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let first_trace = Rc::clone(&trace);
    let nested = right.clone();
    let _left_first = left.subscribe(move |_| {
        first_trace.borrow_mut().push("left-a".to_string());
        nested.send(Event::Add(1))?;
        first_trace.borrow_mut().push(format!(
            "left-return:{}",
            nested.get_snapshot().context.count
        ));
        Ok(())
    });
    let second_trace = Rc::clone(&trace);
    let _left_second = left.subscribe(move |_| {
        second_trace.borrow_mut().push("left-b".to_string());
        Ok(())
    });
    let right_first_trace = Rc::clone(&trace);
    let _right_first = right.subscribe(move |_| {
        right_first_trace.borrow_mut().push("right-a".to_string());
        Ok(())
    });
    let right_second_trace = Rc::clone(&trace);
    let _right_second = right.subscribe(move |_| {
        right_second_trace.borrow_mut().push("right-b".to_string());
        Ok(())
    });

    left.send(Event::Add(1)).unwrap();

    assert_eq!(
        *trace.borrow(),
        ["left-a", "left-b", "right-a", "right-b", "left-return:1"]
    );
}

#[test]
fn subscriber_errors_drain_later_observers_keep_committed_state_and_skip_effects() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let effect_trace = Rc::clone(&trace);
    let store = Store::<i32, ()>::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, (), enqueue| {
            let trace = Rc::clone(&effect_trace);
            enqueue.effect(move |_| {
                trace.borrow_mut().push("effect");
                Ok(())
            });
            Ok(Update::replace(**context + 1))
        },
    ));
    let first_trace = Rc::clone(&trace);
    let first = store.subscribe(move |_| {
        first_trace.borrow_mut().push("bad");
        Err(StoreError::new("first failure"))
    });
    let second_trace = Rc::clone(&trace);
    let second = store.subscribe(move |_| {
        second_trace.borrow_mut().push("good");
        Err(StoreError::new("second failure"))
    });

    let error = store.send(()).unwrap_err();

    assert_eq!(error.to_string(), "first failure");
    assert_eq!(*trace.borrow(), ["bad", "good"]);
    assert_eq!(*store.get_snapshot().context, 1);

    first.unsubscribe();
    second.unsubscribe();
    store.send(()).unwrap();
    assert_eq!(*trace.borrow(), ["bad", "good", "effect"]);
}

#[test]
fn nested_subscriber_failures_still_drain_both_stores() {
    let left = counter();
    let right = counter();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let nested = right.clone();
    let first_trace = Rc::clone(&trace);
    let _left_first = left.subscribe(move |_| {
        first_trace.borrow_mut().push("left-a");
        nested.send(Event::Add(1))
    });
    let second_trace = Rc::clone(&trace);
    let _left_second = left.subscribe(move |_| {
        second_trace.borrow_mut().push("left-b");
        Ok(())
    });
    let right_first_trace = Rc::clone(&trace);
    let _right_first = right.subscribe(move |_| {
        right_first_trace.borrow_mut().push("right-a");
        Err(StoreError::new("nested failure"))
    });
    let right_second_trace = Rc::clone(&trace);
    let _right_second = right.subscribe(move |_| {
        right_second_trace.borrow_mut().push("right-b");
        Ok(())
    });

    let error = left.send(Event::Add(1)).unwrap_err();

    assert_eq!(error.to_string(), "nested failure");
    assert_eq!(*trace.borrow(), ["left-a", "left-b", "right-a", "right-b"]);
    assert_eq!(left.get_snapshot().context.count, 1);
    assert_eq!(right.get_snapshot().context.count, 1);
}
