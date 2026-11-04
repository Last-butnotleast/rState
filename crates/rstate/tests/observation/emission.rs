use std::{cell::RefCell, rc::Rc};

use rstate::{Store, StoreConfig, StoreError, Update};

enum Message {
    Receipt(i32),
    Notice,
}

#[test]
fn filtered_listeners_precede_wildcard_listeners_and_can_unsubscribe() {
    let store = Store::new(StoreConfig::new((), |_, (): &(), enqueue| {
        enqueue.emit(Message::Receipt(7));
        enqueue.emit(Message::Notice);
        Ok(Update::Keep)
    }));
    let trace = Rc::new(RefCell::new(Vec::new()));
    let wildcard_trace = Rc::clone(&trace);
    let _wildcard = store.on_any(move |message| {
        wildcard_trace.borrow_mut().push(match &*message {
            Message::Receipt(value) => format!("any-receipt:{value}"),
            Message::Notice => "any-notice".into(),
        });
        Ok(())
    });
    let specific_trace = Rc::clone(&trace);
    let specific = store.on(
        |message| matches!(message, Message::Receipt(_)),
        move |message| {
            if let Message::Receipt(value) = &*message {
                specific_trace.borrow_mut().push(format!("receipt:{value}"));
            }
            Ok(())
        },
    );

    store.send(()).unwrap();
    assert_eq!(
        *trace.borrow(),
        ["receipt:7", "any-receipt:7", "any-notice"]
    );

    specific.unsubscribe();
    specific.unsubscribe();
    trace.borrow_mut().clear();
    store.send(()).unwrap();
    assert_eq!(*trace.borrow(), ["any-receipt:7", "any-notice"]);
}

#[test]
fn emitted_listener_errors_stop_remaining_listeners_and_effects_after_commit() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let later_trace = Rc::clone(&trace);
    let store = Store::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, (): &(), enqueue| {
            enqueue.emit(Message::Receipt(7));
            let trace = Rc::clone(&later_trace);
            enqueue.effect(move |_| {
                trace.borrow_mut().push("effect");
                Ok(())
            });
            Ok(Update::replace(**context + 1))
        },
    ));
    let listener_trace = Rc::clone(&trace);
    let _bad = store.on(
        |_| true,
        move |_| {
            listener_trace.borrow_mut().push("bad-listener");
            Err(StoreError::new("emit failed"))
        },
    );
    let wildcard_trace = Rc::clone(&trace);
    let _wildcard = store.on_any(move |_| {
        wildcard_trace.borrow_mut().push("wildcard");
        Ok(())
    });

    let error = store.send(()).unwrap_err();

    assert_eq!(error.to_string(), "emit failed");
    assert_eq!(*store.get_snapshot().context, 1);
    assert_eq!(*trace.borrow(), ["bad-listener"]);
}
