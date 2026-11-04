use std::{cell::RefCell, rc::Rc};

use rstate::{Store, StoreConfig, StoreError, Update};

enum Event {
    Start,
    Child,
    Fail,
}

#[test]
fn internal_triggers_publish_once_before_ordered_effects_and_emissions() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let reducer_trace = Rc::clone(&trace);
    let store = Store::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event: &Event, enqueue| {
            match event {
                Event::Start => {
                    let before_trace = Rc::clone(&reducer_trace);
                    enqueue.effect(move |scope| {
                        before_trace
                            .borrow_mut()
                            .push(format!("before:{}", scope.get_snapshot().context));
                        Ok(())
                    });
                    enqueue.trigger(Event::Child);
                    enqueue.emit("parent");
                }
                Event::Child => enqueue.emit("child"),
                Event::Fail => return Err(StoreError::new("child failed")),
            }
            Ok(Update::replace(**context + 1))
        },
    ));
    let observed_trace = Rc::clone(&trace);
    let _subscription = store.subscribe(move |snapshot| {
        observed_trace
            .borrow_mut()
            .push(format!("snapshot:{}", snapshot.context));
        Ok(())
    });
    let emitted_trace = Rc::clone(&trace);
    let _emissions = store.on_any(move |message| {
        emitted_trace.borrow_mut().push(format!("emit:{message}"));
        Ok(())
    });

    store.send(Event::Start).unwrap();

    assert_eq!(
        *trace.borrow(),
        ["snapshot:2", "before:2", "emit:parent", "emit:child"]
    );
}

#[test]
fn failure_in_an_internal_trigger_discards_the_whole_uncommitted_macrostep() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let effect_trace = Rc::clone(&trace);
    let store = Store::<i32, Event>::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event: &Event, enqueue| match event {
            Event::Start => {
                let trace = Rc::clone(&effect_trace);
                enqueue.effect(move |_| {
                    trace.borrow_mut().push("effect");
                    Ok(())
                });
                enqueue.trigger(Event::Child);
                enqueue.trigger(Event::Fail);
                Ok(Update::replace(**context + 1))
            }
            Event::Child => Ok(Update::replace(**context + 1)),
            Event::Fail => Err(StoreError::new("child failed")),
        },
    ));
    let observer_trace = Rc::clone(&trace);
    let _subscription = store.subscribe(move |_| {
        observer_trace.borrow_mut().push("snapshot");
        Ok(())
    });

    let error = store.send(Event::Start).unwrap_err();

    assert_eq!(error.to_string(), "child failed");
    assert_eq!(*store.get_snapshot().context, 0);
    assert!(trace.borrow().is_empty());
}

#[test]
fn enqueue_debug_summarizes_non_debug_events_messages_and_captures() {
    struct Context;
    struct Event;
    struct Message;

    let calls = Rc::new(RefCell::new(0));
    let effect_calls = calls.clone();
    let mut enqueue = rstate::Enqueue::<Context, Event, Message>::default();
    enqueue.trigger(Event);
    enqueue.emit(Message);
    enqueue.effect(move |_| {
        *effect_calls.borrow_mut() += 1;
        Ok(())
    });

    let debug = format!("{enqueue:#?}");

    assert!(debug.contains("queued_events: 1"));
    assert!(debug.contains("Run"));
    assert!(debug.contains("Emit"));
    assert_eq!(*calls.borrow(), 0);
}
