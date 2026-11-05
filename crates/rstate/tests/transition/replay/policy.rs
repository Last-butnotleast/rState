use std::{cell::Cell, rc::Rc};

use rstate::{
    ReplayPolicy, Schema, SchemaCheck, SchemaMetadata, Schemas, Snapshot, Transition,
    TransitionOutcome, Update, ValidationOptions, ValidationStage, replay, replay_with_guard,
};

fn transition() -> Transition<i32, i32> {
    Transition::new(|context: &Rc<i32>, event: &i32, _| {
        Ok(if *event == 0 {
            Update::Keep
        } else {
            Update::replace(context.saturating_add(*event))
        })
    })
    .with_schemas(Schemas::default().with_events(Schema::new(
        SchemaMetadata::new("events"),
        |event: &i32| {
            Ok(if *event < 0 {
                SchemaCheck::invalid(Vec::new())
            } else {
                SchemaCheck::Valid
            })
        },
    )))
    .with_validation(ValidationOptions::default())
}

#[test]
fn stop_includes_the_unavailable_event_without_requesting_a_later_item() {
    let transition = transition();
    let loaded = Rc::new(Snapshot::new(Rc::new(1)));
    let requested = Rc::new(Cell::new(0_u32));
    let counts = requested.clone();
    let events = [2, 0, 3].into_iter().inspect(move |_| {
        counts.set(counts.get().saturating_add(1));
    });

    let result = replay(&transition, loaded.clone(), events, ReplayPolicy::default()).unwrap();

    assert_eq!(*result.snapshot.context, 3);
    assert_eq!(result.outcomes.len(), 2);
    assert!(matches!(
        result.outcomes.last(),
        Some(TransitionOutcome::Unavailable)
    ));
    assert_eq!(requested.get(), 2);
    assert_eq!(*loaded.context, 1);
}

#[test]
fn stop_and_skip_treat_validation_as_an_explicit_non_applied_outcome() {
    let transition = transition();
    let loaded = Rc::new(Snapshot::new(Rc::new(1)));
    let stopped = replay(&transition, loaded.clone(), [2, -1, 3], ReplayPolicy::Stop).unwrap();
    let skipped = replay(
        &transition,
        loaded.clone(),
        [2, 0, -1, 3],
        ReplayPolicy::Skip,
    )
    .unwrap();

    assert_eq!(*stopped.snapshot.context, 3);
    assert_eq!(stopped.outcomes.len(), 2);
    assert_eq!(*skipped.snapshot.context, 6);
    assert_eq!(skipped.outcomes.len(), 4);
    assert!(matches!(
        skipped.outcomes.get(1),
        Some(TransitionOutcome::Unavailable)
    ));
    for outcome in [
        stopped.outcomes.last().unwrap(),
        skipped.outcomes.get(2).unwrap(),
    ] {
        let TransitionOutcome::ValidationFailed { error } = outcome else {
            panic!("expected validation result");
        };
        assert_eq!(
            error.validation_error().unwrap().stage,
            ValidationStage::Event
        );
        assert_eq!(error.dispatch_status(), None);
    }
    assert_eq!(*loaded.context, 1);
}

#[test]
fn guards_observe_the_latest_applied_snapshot_and_rejections_follow_policy() {
    struct Reason(Rc<()>);

    let transition = transition();
    let loaded = Rc::new(Snapshot::new(Rc::new(1)));
    let reason = Rc::new(());
    for policy in [ReplayPolicy::Stop, ReplayPolicy::Skip] {
        let calls = Rc::new(Cell::new(0_u32));
        let observed = calls.clone();
        let result = replay_with_guard(
            &transition,
            loaded.clone(),
            [2, 10, 3],
            policy,
            |snapshot, event| {
                observed.set(observed.get().saturating_add(1));
                assert_eq!(*snapshot.context, if observed.get() == 1 { 1 } else { 3 });
                if *event == 10 {
                    Err(Reason(reason.clone()))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap();
        let TransitionOutcome::Rejected { reason: rejected } = result.outcomes.get(1).unwrap()
        else {
            panic!("expected guard rejection");
        };
        assert!(Rc::ptr_eq(&rejected.0, &reason));
        match policy {
            ReplayPolicy::Stop => {
                assert_eq!(calls.get(), 2);
                assert_eq!(*result.snapshot.context, 3);
                assert_eq!(result.outcomes.len(), 2);
            }
            ReplayPolicy::Skip => {
                assert_eq!(calls.get(), 3);
                assert_eq!(*result.snapshot.context, 6);
                assert_eq!(result.outcomes.len(), 3);
            }
        }
    }
}
