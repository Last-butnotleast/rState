use std::{cell::RefCell, rc::Rc};

use rstate::Inspection;

use super::support::{Event, describe, store};

#[test]
fn inspection_reports_current_state_and_outer_noops_but_not_dry_runs() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    store.send(Event::Add(3)).unwrap();
    let current = store.get_snapshot();
    let retained = Rc::new(RefCell::new(Vec::new()));
    let snapshots = retained.clone();
    let observed = trace.clone();
    let subscription = store
        .inspect(move |inspection| {
            observed.borrow_mut().push(describe(&inspection));
            let snapshot = match inspection {
                Inspection::Initial { snapshot } | Inspection::Transition { snapshot, .. } => {
                    snapshot
                }
            };
            snapshots.borrow_mut().push(snapshot);
            Ok(())
        })
        .unwrap();

    assert_eq!(*trace.borrow(), ["init:3"]);
    assert!(Rc::ptr_eq(&retained.borrow()[0], &current));
    assert!(Rc::ptr_eq(
        &retained.borrow()[0].context.local,
        &current.context.local
    ));
    assert!(store.can(Event::Add(2)).unwrap());
    let pure = store.transition(current.clone(), Event::Add(4)).unwrap();
    assert_eq!(pure.snapshot.context.count, 7);
    assert_eq!(*trace.borrow(), ["init:3"]);

    store.send(Event::Keep).unwrap();
    store.send(Event::Same).unwrap();
    store.send(Event::Unknown).unwrap();
    assert!(store.send(Event::Fail).is_err());
    assert_eq!(
        *trace.borrow(),
        [
            "init:3",
            "inspect:keep:3",
            "inspect:same:3",
            "inspect:unknown:3"
        ]
    );
    assert!(
        retained
            .borrow()
            .iter()
            .all(|value| Rc::ptr_eq(value, &current))
    );

    subscription.unsubscribe();
    subscription.unsubscribe();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(retained.borrow().len(), 4);
    assert_eq!(retained.borrow()[0].context.count, 3);
}

#[test]
fn dropping_the_inspection_handle_does_not_unregister_the_callback() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = store(&trace);
    let observed = trace.clone();
    drop(
        store
            .inspect(move |inspection| {
                observed.borrow_mut().push(describe(&inspection));
                Ok(())
            })
            .unwrap(),
    );

    store.send(Event::Keep).unwrap();

    assert_eq!(*trace.borrow(), ["init:0", "inspect:keep:0"]);
}
