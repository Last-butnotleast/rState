use std::{cell::RefCell, rc::Rc};

use crate::support::{Event, counter};

#[test]
fn subscriptions_have_no_replay_and_only_publish_new_snapshot_identities() {
    let store = counter();
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&values);
    let _subscription = store.subscribe(move |snapshot| {
        observed.borrow_mut().push(snapshot.context.count);
        Ok(())
    });

    assert!(values.borrow().is_empty());
    store.send(Event::Keep).unwrap();
    store.send(Event::Same).unwrap();
    assert!(values.borrow().is_empty());

    store.send(Event::Equal).unwrap();
    store.send(Event::Add(3)).unwrap();
    assert_eq!(*values.borrow(), [0, 3]);
}

#[test]
fn dropping_a_subscription_handle_does_not_unregister_the_observer() {
    let store = counter();
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&values);

    drop(store.subscribe(move |snapshot| {
        observed.borrow_mut().push(snapshot.context.count);
        Ok(())
    }));
    store.send(Event::Add(1)).unwrap();

    assert_eq!(*values.borrow(), [1]);
}

#[test]
fn explicit_unsubscribe_is_idempotent_and_prevents_later_delivery() {
    let store = counter();
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&values);
    let subscription = store.subscribe(move |snapshot| {
        observed.borrow_mut().push(snapshot.context.count);
        Ok(())
    });

    store.send(Event::Add(1)).unwrap();
    subscription.unsubscribe();
    subscription.unsubscribe();
    store.send(Event::Add(1)).unwrap();

    assert_eq!(*values.borrow(), [1]);
}

#[test]
fn subscription_debug_reports_activity_without_running_or_canceling_callbacks() {
    let store = counter();
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = values.clone();
    let subscription = store.subscribe(move |snapshot| {
        observed.borrow_mut().push(snapshot.context.count);
        Ok(())
    });

    assert!(format!("{subscription:?}").contains("active: true"));
    assert!(values.borrow().is_empty());
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*values.borrow(), [1]);

    subscription.unsubscribe();

    assert!(format!("{subscription:#?}").contains("active: false"));
    assert_eq!(*values.borrow(), [1]);
}
