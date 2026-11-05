use std::{cell::Cell, cell::RefCell, rc::Rc};

use rstate::{
    Effect, Enqueue, ReplayPolicy, ReplayResult, Snapshot, Store, StoreConfig, StoreResult,
    Transition, TransitionOutcome, Update, replay, replay_with_guard,
};

enum Event {
    Add(i32),
    Start,
    Same,
}

type FixtureEnqueue = Enqueue<i32, Event, i32>;
type FixtureUpdate = StoreResult<Update<i32>>;
type FixtureTrace = Vec<(i32, Vec<Option<i32>>)>;

fn reducer(runs: Rc<Cell<u32>>) -> impl Fn(&Rc<i32>, &Event, &mut FixtureEnqueue) -> FixtureUpdate {
    move |context, event, enqueue| {
        let effect_runs = runs.clone();
        enqueue.effect(move |_| {
            effect_runs.set(effect_runs.get().saturating_add(1));
            Ok(())
        });
        match event {
            Event::Add(by) => {
                let next = context.saturating_add(*by);
                enqueue.emit(next);
                Ok(Update::replace(next))
            }
            Event::Start => {
                enqueue.trigger(Event::Add(2));
                enqueue.trigger(Event::Add(3));
                enqueue.emit(**context);
                Ok(Update::Keep)
            }
            Event::Same => Ok(Update::Replace(context.clone())),
        }
    }
}

const fn events() -> [Event; 3] {
    [Event::Add(1), Event::Start, Event::Same]
}

fn trace(result: &ReplayResult<i32, Event, i32>) -> Option<FixtureTrace> {
    result
        .outcomes
        .iter()
        .map(|outcome| {
            let TransitionOutcome::Applied { result } = outcome else {
                return None;
            };
            let effects = result
                .effects
                .iter()
                .map(|effect| match effect {
                    Effect::Emit(message) => Some(**message),
                    Effect::Run(_) => None,
                })
                .collect();
            Some((*result.snapshot.context, effects))
        })
        .collect()
}

#[test]
fn replay_matches_store_dispatch_and_sequential_proposals_without_running_effects() {
    let runs = Rc::new(Cell::new(0));
    let transition = Transition::new(reducer(runs.clone()));
    let store = Store::new(StoreConfig::new(10, reducer(runs.clone())));
    let emissions = Rc::new(RefCell::new(Vec::new()));
    let emitted = emissions.clone();
    let _subscription = store.on_any(move |message| {
        emitted.borrow_mut().push(*message);
        Ok(())
    });
    for event in events() {
        store.send(event).unwrap();
    }
    let completed_runs = runs.get();
    assert_eq!(completed_runs, 5);
    let loaded = Rc::new(Snapshot::new(Rc::new(10)));
    let replayed = replay(&transition, loaded.clone(), events(), ReplayPolicy::Stop).unwrap();
    let repeated = replay(&transition, loaded.clone(), events(), ReplayPolicy::Stop).unwrap();
    let mut current = loaded.clone();
    let mut sequential = Vec::new();
    for event in events() {
        let proposal = transition.transition(&current, event).unwrap();
        current = proposal.snapshot.clone();
        sequential.push(proposal);
    }

    assert_eq!(*replayed.snapshot.context, *store.get_snapshot().context);
    assert_eq!(*replayed.snapshot.context, *current.context);
    assert_eq!(trace(&replayed), trace(&repeated));
    assert_eq!(
        trace(&replayed),
        Some(vec![
            (11, vec![None, Some(11)]),
            (16, vec![None, Some(11), None, Some(13), None, Some(16)]),
            (16, vec![None]),
        ])
    );
    for (outcome, proposal) in replayed.outcomes.iter().zip(&sequential) {
        let TransitionOutcome::Applied { result } = outcome else {
            panic!("expected proposed state");
        };
        assert_eq!(*result.snapshot.context, *proposal.snapshot.context);
        assert_eq!(result.allowed, proposal.allowed);
    }
    let actual: Vec<_> = replayed
        .outcomes
        .iter()
        .flat_map(|outcome| match outcome {
            TransitionOutcome::Applied { result } => result.effects.iter(),
            _ => panic!("expected proposed effects"),
        })
        .filter_map(|effect| match effect {
            Effect::Emit(message) => Some(**message),
            Effect::Run(_) => None,
        })
        .collect();
    assert_eq!(actual, *emissions.borrow());
    assert_eq!(runs.get(), completed_runs);
    assert_eq!(*loaded.context, 10);
    assert_eq!(replayed.snapshot.history_status().limit, None);
    assert!(format!("{replayed:?}").contains("ReplayResult"));
    let TransitionOutcome::Applied { result } = replayed.outcomes.last().unwrap() else {
        panic!("expected explicit same-context proposal");
    };
    assert!(Rc::ptr_eq(&result.snapshot, &replayed.snapshot));
}

#[test]
fn replay_retains_empty_and_same_context_snapshot_identity_without_trait_bounds() {
    struct Context(Rc<()>);
    struct Event;
    struct Message;

    let transition: Transition<Context, Event, Message> =
        Transition::new(|context: &Rc<Context>, _, _| Ok(Update::Replace(context.clone())));
    let loaded = Rc::new(Snapshot::new(Rc::new(Context(Rc::new(())))));
    let empty = replay(
        &transition,
        loaded.clone(),
        std::iter::empty(),
        ReplayPolicy::default(),
    )
    .unwrap();
    assert!(Rc::ptr_eq(&empty.snapshot, &loaded));
    assert!(empty.outcomes.is_empty());

    let replayed = replay(&transition, loaded.clone(), [Event], ReplayPolicy::Skip).unwrap();
    assert!(Rc::ptr_eq(&replayed.snapshot, &loaded));
    assert!(Rc::ptr_eq(&replayed.snapshot.context.0, &loaded.context.0));
    assert_eq!(replayed.outcomes.len(), 1);
}

#[test]
fn replay_context_events_messages_and_reasons_can_borrow_local_values() {
    let text = String::from("borrowed");
    let transition: Transition<&str, &str, &str> = Transition::new(|context, event, enqueue| {
        assert_eq!(**context, *event);
        enqueue.emit(*event);
        Ok(Update::Keep)
    });
    let loaded = Rc::new(Snapshot::new(Rc::new(text.as_str())));
    let reason = String::from("unavailable");
    let replayed = replay_with_guard(
        &transition,
        loaded.clone(),
        [text.as_str(), "other", text.as_str()],
        ReplayPolicy::Skip,
        |_, event| {
            if *event == "other" {
                Err(reason.as_str())
            } else {
                Ok(())
            }
        },
    )
    .unwrap();

    assert!(Rc::ptr_eq(&replayed.snapshot, &loaded));
    assert!(
        matches!(replayed.outcomes.get(1), Some(TransitionOutcome::Rejected { reason: value }) if *value == reason)
    );
    assert!(matches!(
        replayed.outcomes.last(),
        Some(TransitionOutcome::Applied { .. })
    ));
}
