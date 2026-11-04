use std::{cell::Cell, rc::Rc};

use rstate::{StoreConfig, Update};

#[test]
fn config_debug_omits_callbacks_without_requiring_event_or_message_debug() {
    #[derive(Debug)]
    struct Context(i32);
    struct Event;
    struct Message;

    let calls = Rc::new(Cell::new(0));
    let reducer_calls = calls.clone();
    let config = StoreConfig::<Context, Event, Message>::new(Context(42), move |context, _, _| {
        reducer_calls.set(reducer_calls.get() + context.0);
        Ok(Update::Keep)
    });

    let debug = format!("{config:#?}");

    assert!(debug.contains("StoreConfig"));
    assert!(debug.contains("42"));
    assert_eq!(calls.get(), 0);
}
