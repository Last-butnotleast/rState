use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{Inspection, Store, StoreConfig, Update};

#[derive(Debug)]
struct Event {
    value: i32,
}

#[test]
fn inspection_debug_formats_borrowed_events_and_shared_snapshots() {
    let reducer_event = Rc::new(Cell::new(std::ptr::null::<Event>()));
    let reduced = reducer_event.clone();
    let store = Store::<i32, Event>::new(StoreConfig::new(0, move |_, event, _| {
        reduced.set(std::ptr::from_ref(event));
        Ok(Update::Keep)
    }));
    let initial_snapshot = store.get_snapshot();
    let formatted = Rc::new(RefCell::new(Vec::new()));
    let recorded = formatted.clone();
    let shared_initial = Rc::new(Cell::new(false));
    let initial = shared_initial.clone();
    let same_event = Rc::new(Cell::new(false));
    let borrowed = same_event.clone();
    let subscription = store
        .inspect(move |inspection| {
            recorded.borrow_mut().push(format!("{inspection:?}"));
            match inspection {
                Inspection::Initial { snapshot } => {
                    initial.set(Rc::ptr_eq(&snapshot, &initial_snapshot));
                }
                Inspection::Transition { event, .. } => {
                    borrowed.set(std::ptr::eq(std::ptr::from_ref(event), reducer_event.get()));
                    recorded.borrow_mut().push(format!("value:{}", event.value));
                }
            }
            Ok(())
        })
        .unwrap();

    store.send(Event { value: 7 }).unwrap();

    let formatted = formatted.borrow();
    assert!(formatted[0].contains("Initial"));
    assert!(formatted[1].contains("Transition"));
    assert!(formatted[1].contains("value: 7"));
    assert_eq!(formatted[2], "value:7");
    assert!(shared_initial.get());
    assert!(same_event.get());
    subscription.unsubscribe();
}
