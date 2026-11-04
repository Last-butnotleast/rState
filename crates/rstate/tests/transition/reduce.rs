use std::{cell::RefCell, rc::Rc};

use rstate::{Effect, Store, StoreConfig, Update};

enum Event {
    Start,
    Child(i32),
    Grandchild,
    EffectOnly,
    TriggerOnly,
}

#[test]
fn internal_triggers_are_breadth_first_and_collect_parent_effects_first() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let reductions = Rc::clone(&order);
    let store = Store::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event: &Event, enqueue| {
            match event {
                Event::Start => {
                    reductions.borrow_mut().push("start");
                    enqueue.emit("before");
                    enqueue.trigger(Event::Child(1));
                    enqueue.trigger(Event::Child(2));
                    enqueue.emit("after");
                }
                Event::Child(1) => {
                    reductions.borrow_mut().push("child-one");
                    enqueue.trigger(Event::Grandchild);
                    enqueue.emit("child-one");
                }
                Event::Child(_) => {
                    reductions.borrow_mut().push("child-two");
                    enqueue.emit("child-two");
                }
                Event::Grandchild => {
                    reductions.borrow_mut().push("grandchild");
                    enqueue.emit("grandchild");
                }
                Event::EffectOnly | Event::TriggerOnly => {}
            }
            Ok(Update::replace(**context + 1))
        },
    ));

    let result = store
        .transition(store.get_snapshot(), Event::Start)
        .unwrap();
    let emitted: Vec<_> = result
        .effects
        .iter()
        .map(|effect| match effect {
            Effect::Emit(message) => **message,
            Effect::Run(_) => panic!("expected emitted message"),
        })
        .collect();

    assert_eq!(
        *order.borrow(),
        ["start", "child-one", "child-two", "grandchild"]
    );
    assert_eq!(
        emitted,
        ["before", "after", "child-one", "child-two", "grandchild"]
    );
    assert_eq!(*result.snapshot.context, 4);
    assert_eq!(*store.get_snapshot().context, 0);
}

#[test]
fn can_collects_effects_and_triggers_without_executing_or_publishing_them() {
    let runs = Rc::new(RefCell::new(0));
    let effect_runs = Rc::clone(&runs);
    let store = Store::<i32, Event>::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, event: &Event, enqueue| match event {
            Event::EffectOnly => {
                let runs = Rc::clone(&effect_runs);
                enqueue.effect(move |_| {
                    *runs.borrow_mut() += 1;
                    Ok(())
                });
                Ok(Update::Keep)
            }
            Event::TriggerOnly => {
                enqueue.trigger(Event::Child(3));
                Ok(Update::Keep)
            }
            Event::Child(by) => Ok(Update::replace(**context + by)),
            _ => Ok(Update::Keep),
        },
    ));

    assert!(store.can(Event::EffectOnly).unwrap());
    assert!(store.can(Event::TriggerOnly).unwrap());
    assert_eq!(*runs.borrow(), 0);
    assert_eq!(*store.get_snapshot().context, 0);
}
