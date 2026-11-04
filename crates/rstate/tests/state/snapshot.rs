use std::rc::Rc;

use crate::support::{Event, counter};

#[test]
fn keep_and_same_context_preserve_snapshot_identity() {
    let store = counter();
    let initial = store.get_snapshot();

    store.send(Event::Keep).unwrap();
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));

    store.send(Event::Same).unwrap();
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    assert!(Rc::ptr_eq(&initial.context, &store.get_snapshot().context));
}

#[test]
fn equal_context_replacement_has_new_snapshot_and_context_identity() {
    let store = counter();
    let initial = store.get_snapshot();

    store.send(Event::Equal).unwrap();
    let current = store.get_snapshot();

    assert_eq!(initial.context, current.context);
    assert!(!Rc::ptr_eq(&initial, &current));
    assert!(!Rc::ptr_eq(&initial.context, &current.context));
}

#[test]
fn cloned_store_handles_share_state_while_initial_snapshot_is_retained() {
    let store = counter();
    let alias = store.clone();
    let initial = store.get_initial_snapshot();

    alias.send(Event::Add(4)).unwrap();

    assert_eq!(store.get_snapshot().context.count, 4);
    assert!(Rc::ptr_eq(&store.get_snapshot(), &alias.get_snapshot()));
    assert!(Rc::ptr_eq(&initial, &store.get_initial_snapshot()));
    assert_eq!(initial.context.count, 0);
}
