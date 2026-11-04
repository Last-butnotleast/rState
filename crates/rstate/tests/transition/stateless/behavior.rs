use std::{cell::Cell, rc::Rc};

use rstate::{Effect, Enqueue, Snapshot, Store, StoreConfig, StoreResult, Transition, Update};

#[test]
fn external_snapshots_are_independent_and_handles_add_no_trait_bounds() {
    struct Context {
        count: i32,
        local: Rc<()>,
    }
    enum Event {
        Add(i32),
        Keep,
        Same,
    }
    struct Message;

    let transition = Transition::<Context, Event, Message>::new(|context, event, _| {
        Ok(match event {
            Event::Add(by) => Update::replace(Context {
                count: context.count.saturating_add(*by),
                local: context.local.clone(),
            }),
            Event::Keep => Update::Keep,
            Event::Same => Update::Replace(context.clone()),
        })
    });
    let cloned = transition.clone();
    assert!(format!("{cloned:?}").contains("Transition"));
    let first = Rc::new(Snapshot::new(Rc::new(Context {
        count: 1,
        local: Rc::new(()),
    })));
    let second = Rc::new(Snapshot::new(Rc::new(Context {
        count: 10,
        local: Rc::new(()),
    })));

    let added = transition.transition(&first, Event::Add(2)).unwrap();
    let repeated = cloned.transition(&first, Event::Add(2)).unwrap();
    let independent = transition.transition(&second, Event::Add(2)).unwrap();
    assert_eq!(added.snapshot.context.count, 3);
    assert_eq!(repeated.snapshot.context.count, 3);
    assert_eq!(independent.snapshot.context.count, 12);
    assert!(added.allowed);
    assert!(added.effects.is_empty());
    assert_eq!(first.context.count, 1);
    assert_eq!(second.context.count, 10);
    assert!(Rc::ptr_eq(
        &added.snapshot.context.local,
        &first.context.local
    ));

    let kept = transition.transition(&first, Event::Keep).unwrap();
    let same = transition.transition(&first, Event::Same).unwrap();
    assert!(Rc::ptr_eq(&kept.snapshot, &first));
    assert!(Rc::ptr_eq(&same.snapshot, &first));
    assert!(!kept.allowed);
    assert!(same.allowed);
}

enum Event {
    Start(Rc<Cell<u32>>),
    Child(bool),
    Grandchild,
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared fixture uses the fallible reducer signature"
)]
fn reducer(
    context: &Rc<i32>,
    event: &Event,
    enqueue: &mut Enqueue<i32, Event, &'static str>,
) -> StoreResult<Update<i32>> {
    match event {
        Event::Start(runs) => {
            let runs = runs.clone();
            enqueue.effect(move |_| {
                runs.set(runs.get().saturating_add(1));
                Ok(())
            });
            enqueue.emit("before");
            enqueue.trigger(Event::Child(true));
            enqueue.trigger(Event::Child(false));
            enqueue.emit("after");
        }
        Event::Child(true) => {
            enqueue.trigger(Event::Grandchild);
            enqueue.emit("child-one");
        }
        Event::Child(false) => enqueue.emit("child-two"),
        Event::Grandchild => enqueue.emit("grandchild"),
    }
    Ok(Update::replace(context.saturating_add(1)))
}

#[test]
fn stateless_and_store_queries_share_fifo_and_ordered_inert_effects() {
    let transition = Transition::new(reducer);
    let store = Store::new(StoreConfig::new(-100, reducer));
    let store_initial = store.get_snapshot();
    let snapshot = Rc::new(Snapshot::new(Rc::new(10)));
    let runs = Rc::new(Cell::new(0));

    for result in [
        transition
            .transition(&snapshot, Event::Start(runs.clone()))
            .unwrap(),
        transition
            .transition(&snapshot, Event::Start(runs.clone()))
            .unwrap(),
        store
            .transition(snapshot.clone(), Event::Start(runs.clone()))
            .unwrap(),
    ] {
        assert_eq!(*result.snapshot.context, 14);
        assert!(result.allowed);
        let output: Vec<_> = result
            .effects
            .iter()
            .map(|effect| match effect {
                Effect::Run(_) => "callback",
                Effect::Emit(message) => **message,
            })
            .collect();
        assert_eq!(
            output,
            [
                "callback",
                "before",
                "after",
                "child-one",
                "child-two",
                "grandchild"
            ]
        );
    }

    assert_eq!(runs.get(), 0);
    assert_eq!(*snapshot.context, 10);
    assert!(Rc::ptr_eq(&store_initial, &store.get_snapshot()));
}

#[test]
fn supplied_contexts_and_events_can_borrow_local_values() {
    let text = String::from("local");
    let snapshot = Rc::new(Snapshot::new(Rc::new(text.as_str())));
    let transition: Transition<&str, &str> = Transition::new(|context, event, _| {
        assert_eq!(**context, *event);
        Ok(Update::Keep)
    });

    let result = transition.transition(&snapshot, text.as_str()).unwrap();
    assert!(Rc::ptr_eq(&snapshot, &result.snapshot));
}
