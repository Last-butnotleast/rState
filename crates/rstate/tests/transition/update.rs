use std::rc::Rc;

use crate::support::{Event, counter};

#[test]
fn can_distinguishes_keep_from_explicit_same_context_replacement() {
    let store = counter();
    let initial = store.get_snapshot();

    assert!(!store.can(Event::Keep).unwrap());
    assert!(store.can(Event::Same).unwrap());
    assert!(store.can(Event::Equal).unwrap());
    assert!(store.can(Event::Add(1)).unwrap());
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
}

#[test]
fn transition_accepts_an_explicit_snapshot_without_mutating_live_state() {
    let store = counter();
    let initial = store.get_snapshot();
    store.send(Event::Add(10)).unwrap();
    let live = store.get_snapshot();

    let result = store
        .transition(Rc::clone(&initial), Event::Add(2))
        .unwrap();

    assert_eq!(result.snapshot.context.count, 2);
    assert!(result.allowed);
    assert!(result.effects.is_empty());
    assert!(Rc::ptr_eq(&live, &store.get_snapshot()));
    assert_eq!(store.get_snapshot().context.count, 10);
    assert_eq!(initial.context.count, 0);
}

#[test]
fn reducer_failure_keeps_the_live_and_initial_snapshots() {
    let store = counter();
    let initial = store.get_snapshot();

    let error = store.send(Event::Fail).unwrap_err();

    assert_eq!(error.to_string(), "reducer failed");
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    assert_eq!(
        store.can(Event::Fail).unwrap_err().to_string(),
        "reducer failed"
    );
}
