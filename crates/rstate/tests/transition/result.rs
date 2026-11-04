use std::{cell::Cell, rc::Rc};

use rstate::{Store, StoreConfig, StoreError, Update};

enum Event {
    Fail,
    Effects,
}

struct Message;

#[expect(
    clippy::arithmetic_side_effects,
    reason = "the fixture uses bounded counter inputs"
)]
fn store(runs: Rc<Cell<usize>>) -> Store<i32, Event, Message> {
    Store::new(StoreConfig::new(0, move |_, event, enqueue| {
        match event {
            Event::Fail => return Err(StoreError::new("failed transition")),
            Event::Effects => {
                let runs = runs.clone();
                enqueue.effect(move |_| {
                    runs.set(runs.get() + 1);
                    Ok(())
                });
                enqueue.emit(Message);
            }
        }
        Ok(Update::Keep)
    }))
}

#[test]
fn transition_errors_support_unwrap_err_without_event_or_message_debug() {
    let store = store(Rc::new(Cell::new(0)));

    let error = store
        .transition(store.get_snapshot(), Event::Fail)
        .unwrap_err();

    assert_eq!(error.to_string(), "failed transition");
}

#[test]
fn result_debug_summarizes_opaque_effects_without_running_them() {
    let runs = Rc::new(Cell::new(0));
    let store = store(runs.clone());
    let result = store
        .transition(store.get_snapshot(), Event::Effects)
        .unwrap();

    let debug = format!("{result:#?}");

    assert!(debug.contains("TransitionResult"));
    assert!(debug.contains("Run"));
    assert!(debug.contains("Emit"));
    assert_eq!(runs.get(), 0);
}
