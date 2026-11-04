mod combine;
mod map;

use std::{cell::RefCell, rc::Rc};

use rstate::{Store, StoreConfig, Update};

struct Context {
    count: i32,
    label: &'static str,
}

struct Selected {
    count: i32,
}

enum Event {
    Add(i32),
    Label,
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "the fixture uses bounded counter inputs"
)]
fn store() -> Store<Context, Event> {
    Store::new(StoreConfig::new(
        Context {
            count: 0,
            label: "before",
        },
        |context: &Rc<Context>, event: &Event, _| {
            Ok(Update::replace(match event {
                Event::Add(by) => Context {
                    count: context.count + by,
                    label: context.label,
                },
                Event::Label => Context {
                    count: context.count,
                    label: "after",
                },
            }))
        },
    ))
}

#[test]
fn unobserved_selections_are_lazy_and_retain_equal_cached_values() {
    let store = store();
    let calls = Rc::new(RefCell::new(0));
    let computations = Rc::clone(&calls);
    let selected = store.select(
        move |context| {
            *computations.borrow_mut() += 1;
            Selected {
                count: context.count,
            }
        },
        |left, right| left.count == right.count,
    );

    assert_eq!(*calls.borrow(), 0);
    let first = selected.get();
    assert_eq!(first.count, 0);
    assert!(Rc::ptr_eq(&first, &selected.get()));
    assert_eq!(*calls.borrow(), 1);

    store.send(Event::Label).unwrap();
    assert_eq!(*calls.borrow(), 1);
    assert!(Rc::ptr_eq(&first, &selected.get()));
    assert_eq!(*calls.borrow(), 2);

    store.send(Event::Add(2)).unwrap();
    store.send(Event::Add(3)).unwrap();
    assert_eq!(*calls.borrow(), 2);
    assert_eq!(selected.get().count, 5);
    assert_eq!(*calls.borrow(), 3);
}

#[test]
fn selected_subscriptions_suppress_unrelated_updates_and_stop_explicitly() {
    let store = store();
    let selected = store.select(|context| context.count, |left, right| left == right);
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&values);
    let subscription = selected.subscribe(move |value| {
        observed.borrow_mut().push(*value);
        Ok(())
    });

    assert!(values.borrow().is_empty());
    store.send(Event::Label).unwrap();
    assert!(values.borrow().is_empty());
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*values.borrow(), [1]);

    subscription.unsubscribe();
    store.send(Event::Add(2)).unwrap();
    assert_eq!(*values.borrow(), [1]);
    assert_eq!(*selected.get(), 3);
}

#[test]
fn selected_callbacks_can_send_and_reconcile_before_the_nested_send_returns() {
    let store = store();
    let selected = store.select(|context| context.count, |left, right| left == right);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let selected_trace = Rc::clone(&trace);
    let nested = store.clone();
    let selected_read = selected.clone();
    let _selected = selected.subscribe(move |value| {
        selected_trace
            .borrow_mut()
            .push(format!("selected:{value}"));
        if *value == 1 {
            nested.send(Event::Add(1))?;
            selected_trace
                .borrow_mut()
                .push(format!("selected-return:{}", selected_read.get()));
        }
        Ok(())
    });
    let direct_trace = Rc::clone(&trace);
    let _direct = store.subscribe(move |snapshot| {
        direct_trace
            .borrow_mut()
            .push(format!("direct:{}", snapshot.context.count));
        Ok(())
    });

    store.send(Event::Add(1)).unwrap();

    assert_eq!(
        *trace.borrow(),
        ["selected:1", "direct:2", "selected-return:2"]
    );
    assert_eq!(*selected.get(), 2);
}

#[test]
fn selection_debug_does_not_evaluate_or_observe_a_non_debug_value() {
    let store = store();
    let calls = Rc::new(RefCell::new(0));
    let computations = calls.clone();
    let selected = store.select(
        move |context| {
            *computations.borrow_mut() += 1;
            Selected {
                count: context.count,
            }
        },
        |left, right| left.count == right.count,
    );

    assert!(format!("{selected:?}").contains("Selection"));
    assert_eq!(*calls.borrow(), 0);

    let subscription = selected.subscribe(|_| Ok(()));
    assert_eq!(*calls.borrow(), 1);
    assert!(format!("{selected:#?}").contains("Selection"));
    assert_eq!(*calls.borrow(), 1);
    subscription.unsubscribe();
}
