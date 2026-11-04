use std::{
    cell::{Cell, RefCell},
    error::Error,
    io,
    rc::Rc,
};

use rstate::{Enqueue, StoreError, StoreFactory, StoreResult, Update};

struct Input {
    count: i32,
    label: String,
    marker: Rc<()>,
}

struct Context {
    count: i32,
    label: String,
    marker: Rc<()>,
}

enum Event {
    Add(i32, Rc<()>),
    Emit,
    Keep,
}

struct Message {
    count: i32,
    marker: Rc<()>,
}

struct Selected {
    count: i32,
}

fn input(count: i32, label: &str) -> Input {
    Input {
        count,
        label: label.into(),
        marker: Rc::new(()),
    }
}

fn reduce(
    context: &Rc<Context>,
    event: &Event,
    enqueue: &mut Enqueue<Context, Event, Message>,
) -> StoreResult<Update<Context>> {
    match event {
        Event::Add(by, marker) => Ok(Update::replace(Context {
            count: context.count + by,
            label: context.label.clone(),
            marker: Rc::clone(marker),
        })),
        Event::Emit => {
            enqueue.emit(Message {
                count: context.count,
                marker: Rc::clone(&context.marker),
            });
            Ok(Update::Keep)
        }
        Event::Keep => Ok(Update::Keep),
    }
}

fn factory(calls: Rc<Cell<usize>>) -> StoreFactory<Input, Context, Event, Message> {
    StoreFactory::new(
        move |input: Input| {
            calls.set(calls.get() + 1);
            Ok(Context {
                count: input.count,
                label: input.label,
                marker: input.marker,
            })
        },
        reduce,
    )
}

#[test]
fn cloned_factories_initialize_each_consumed_non_clone_input() {
    let calls = Rc::new(Cell::new(0));
    let factory = factory(Rc::clone(&calls));
    let cloned = factory.clone();
    let left = factory.create(input(2, "left")).unwrap();
    let right = cloned.create(input(9, "right")).unwrap();

    assert_eq!(calls.get(), 2);
    assert_eq!(left.get_snapshot().context.count, 2);
    assert_eq!(left.get_snapshot().context.label, "left");
    assert_eq!(right.get_snapshot().context.count, 9);
    assert_eq!(right.get_snapshot().context.label, "right");
    assert!(!Rc::ptr_eq(
        &left.get_snapshot().context.marker,
        &right.get_snapshot().context.marker,
    ));
}

#[test]
fn equal_inputs_create_independent_state_observers_emissions_and_selection_caches() {
    let factory = factory(Rc::new(Cell::new(0)));
    let left = factory.create(input(5, "same")).unwrap();
    let right = factory.create(input(5, "same")).unwrap();
    let left_initial = left.get_initial_snapshot();
    let right_initial = right.get_initial_snapshot();
    assert!(!Rc::ptr_eq(&left_initial, &right_initial));
    assert!(!Rc::ptr_eq(&left_initial.context, &right_initial.context));
    assert!(Rc::ptr_eq(&left_initial, &left.get_snapshot()));

    let left_selected = left.select(
        |context| Selected {
            count: context.count,
        },
        |left, right| left.count == right.count,
    );
    let right_selected = right.select(
        |context| Selected {
            count: context.count,
        },
        |left, right| left.count == right.count,
    );
    let left_cached = left_selected.get();
    let right_cached = right_selected.get();
    assert!(!Rc::ptr_eq(&left_cached, &right_cached));

    let observations = Rc::new(RefCell::new(Vec::new()));
    let left_observations = Rc::clone(&observations);
    let left_subscription = left.subscribe(move |snapshot| {
        left_observations
            .borrow_mut()
            .push(("left", snapshot.context.count));
        Ok(())
    });
    let right_observations = Rc::clone(&observations);
    let right_subscription = right.subscribe(move |snapshot| {
        right_observations
            .borrow_mut()
            .push(("right", snapshot.context.count));
        Ok(())
    });
    let selected = Rc::new(RefCell::new(Vec::new()));
    let left_values = Rc::clone(&selected);
    let left_selection_subscription = left_selected.subscribe(move |value| {
        left_values.borrow_mut().push(("left", value.count));
        Ok(())
    });
    let right_values = Rc::clone(&selected);
    let right_selection_subscription = right_selected.subscribe(move |value| {
        right_values.borrow_mut().push(("right", value.count));
        Ok(())
    });
    let emissions = Rc::new(RefCell::new(Vec::new()));
    let left_emissions = Rc::clone(&emissions);
    let left_messages = left.on_any(move |message| {
        left_emissions
            .borrow_mut()
            .push(("left", message.count, Rc::clone(&message.marker)));
        Ok(())
    });
    let right_emissions = Rc::clone(&emissions);
    let right_messages = right.on_any(move |message| {
        right_emissions
            .borrow_mut()
            .push(("right", message.count, Rc::clone(&message.marker)));
        Ok(())
    });

    left.send(Event::Add(2, Rc::new(()))).unwrap();
    left.send(Event::Emit).unwrap();
    assert_eq!(*observations.borrow(), [("left", 7)]);
    assert_eq!(*selected.borrow(), [("left", 7)]);
    assert_eq!(right.get_snapshot().context.count, 5);
    assert!(Rc::ptr_eq(&right_initial, &right.get_snapshot()));
    assert!(Rc::ptr_eq(&right_cached, &right_selected.get()));
    assert_eq!(emissions.borrow().len(), 1);
    assert_eq!(emissions.borrow()[0].0, "left");
    assert_eq!(emissions.borrow()[0].1, 7);
    assert!(Rc::ptr_eq(
        &emissions.borrow()[0].2,
        &left.get_snapshot().context.marker
    ));

    right.send(Event::Add(3, Rc::new(()))).unwrap();
    right.send(Event::Emit).unwrap();
    assert_eq!(*observations.borrow(), [("left", 7), ("right", 8)]);
    assert_eq!(*selected.borrow(), [("left", 7), ("right", 8)]);
    assert_eq!(left.get_initial_snapshot().context.count, 5);
    assert_eq!(right.get_initial_snapshot().context.count, 5);
    assert_eq!(left.get_snapshot().context.count, 7);
    assert_eq!(emissions.borrow().len(), 2);
    assert_eq!(emissions.borrow()[1].0, "right");

    left_subscription.unsubscribe();
    left_selection_subscription.unsubscribe();
    left_messages.unsubscribe();
    left.send(Event::Add(1, Rc::new(()))).unwrap();
    left.send(Event::Emit).unwrap();
    right.send(Event::Add(1, Rc::new(()))).unwrap();
    right.send(Event::Emit).unwrap();
    assert_eq!(
        *observations.borrow(),
        [("left", 7), ("right", 8), ("right", 9)]
    );
    assert_eq!(
        *selected.borrow(),
        [("left", 7), ("right", 8), ("right", 9)]
    );
    assert_eq!(emissions.borrow().len(), 3);
    assert_eq!(emissions.borrow()[2].0, "right");
    right_subscription.unsubscribe();
    right_selection_subscription.unsubscribe();
    right_messages.unsubscribe();
}

#[test]
fn failed_initialization_preserves_sources_and_leaves_the_factory_reusable() {
    let initializer_calls = Rc::new(Cell::new(0));
    let reducer_calls = Rc::new(Cell::new(0));
    let initialized = Rc::clone(&initializer_calls);
    let reduced = Rc::clone(&reducer_calls);
    let factory: StoreFactory<i32, i32, i32> = StoreFactory::new(
        move |value: i32| {
            initialized.set(initialized.get() + 1);
            if value < 0 {
                Err(StoreError::from_error(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "negative initial count",
                )))
            } else {
                Ok(value)
            }
        },
        move |context: &Rc<i32>, by: &i32, _| {
            reduced.set(reduced.get() + 1);
            Ok(Update::replace(**context + by))
        },
    );
    let existing = factory.create(2).unwrap();
    let observed = Rc::new(Cell::new(0));
    let notifications = Rc::clone(&observed);
    let subscription = existing.subscribe(move |_| {
        notifications.set(notifications.get() + 1);
        Ok(())
    });

    let error = factory.create(-1).unwrap_err();
    assert_eq!(error.to_string(), "negative initial count");
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    assert_eq!(reducer_calls.get(), 0);
    assert_eq!(observed.get(), 0);
    assert_eq!(*existing.get_snapshot().context, 2);

    let recovered = factory.create(9).unwrap();
    recovered.send(1).unwrap();
    assert_eq!(initializer_calls.get(), 3);
    assert_eq!(reducer_calls.get(), 1);
    assert_eq!(*recovered.get_snapshot().context, 10);
    assert_eq!(observed.get(), 0);
    subscription.unsubscribe();
}

#[test]
fn created_stores_keep_the_shared_reducer_alive_after_factory_handles_are_dropped() {
    let reduced = Rc::new(Cell::new(0));
    let calls = Rc::clone(&reduced);
    let factory: StoreFactory<i32, i32, i32> = StoreFactory::new(
        |initial: i32| Ok(initial),
        move |context: &Rc<i32>, by: &i32, _| {
            calls.set(calls.get() + 1);
            Ok(Update::replace(**context + by))
        },
    );
    let cloned = factory.clone();
    let left = factory.create(1).unwrap();
    let right = cloned.create(10).unwrap();
    drop(factory);
    drop(cloned);

    left.send(2).unwrap();
    right.send(3).unwrap();
    assert_eq!(*left.get_snapshot().context, 3);
    assert_eq!(*right.get_snapshot().context, 13);
    assert_eq!(reduced.get(), 2);
}

#[test]
fn unit_inputs_create_ordinary_stores_with_pure_transition_and_availability() {
    let factory: StoreFactory<(), i32, ()> = StoreFactory::new(
        |()| Ok(1),
        |context: &Rc<i32>, _: &(), _| Ok(Update::replace(**context + 1)),
    );
    let store = factory.create(()).unwrap();
    let initial = store.get_snapshot();

    assert!(store.can(()).unwrap());
    let transition = store.transition(Rc::clone(&initial), ()).unwrap();
    assert_eq!(*transition.snapshot.context, 2);
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    store.send(()).unwrap();
    assert_eq!(*store.get_snapshot().context, 2);
}

#[test]
fn factory_debug_accepts_opaque_types_without_executing_callbacks() {
    let initialized = Rc::new(Cell::new(0));
    let reduced = Rc::new(Cell::new(0));
    let initializer_calls = Rc::clone(&initialized);
    let reducer_calls = Rc::clone(&reduced);
    let factory = StoreFactory::new(
        move |input: Input| {
            initializer_calls.set(initializer_calls.get() + 1);
            Ok(Context {
                count: input.count,
                label: input.label,
                marker: input.marker,
            })
        },
        move |context, event, enqueue| {
            reducer_calls.set(reducer_calls.get() + 1);
            reduce(context, event, enqueue)
        },
    );

    assert!(format!("{factory:?}").contains("StoreFactory"));
    assert!(format!("{:#?}", factory.clone()).contains("StoreFactory"));
    assert_eq!(initialized.get(), 0);
    assert_eq!(reduced.get(), 0);

    let store = factory.create(input(2, "opaque")).unwrap();
    assert!(!store.can(Event::Keep).unwrap());
    assert_eq!(initialized.get(), 1);
    assert_eq!(reduced.get(), 1);
}

#[test]
fn borrowed_inputs_can_be_converted_to_owned_contexts() {
    let label = String::from("borrowed input");
    let factory: StoreFactory<&str, String, ()> = StoreFactory::new(
        |input: &str| Ok(input.to_owned()),
        |_, _: &(), _| Ok(Update::Keep),
    );
    let store = factory.clone().create(label.as_str()).unwrap();
    drop(factory);
    drop(label);

    assert_eq!(*store.get_snapshot().context, "borrowed input");
}
