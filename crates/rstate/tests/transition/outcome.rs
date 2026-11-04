use std::{cell::Cell, error::Error, rc::Rc};

use rstate::{
    Schema, SchemaCheck, SchemaMetadata, Schemas, Snapshot, Store, StoreConfig, StoreError,
    Transition, TransitionOutcome, Update, ValidationError, ValidationOptions, ValidationStage,
};

enum Event {
    Keep,
    Same,
    Emit,
    Invalid,
    InvalidContext,
    InvalidCommand,
}

#[test]
fn outcomes_distinguish_unavailable_events_from_same_identity_and_command_only_application() {
    let transition = Transition::new(|context: &Rc<i32>, event: &Event, enqueue| {
        Ok(match event {
            Event::Keep => Update::Keep,
            Event::Same => Update::Replace(context.clone()),
            _ => {
                enqueue.emit("command");
                Update::Keep
            }
        })
    });
    let snapshot = Rc::new(Snapshot::new(Rc::new(1)));
    assert!(matches!(
        transition.outcome(&snapshot, Event::Keep).unwrap(),
        TransitionOutcome::Unavailable
    ));
    let TransitionOutcome::Applied { result } = transition.outcome(&snapshot, Event::Same).unwrap()
    else {
        panic!("explicit replacement must be applied");
    };
    assert!(Rc::ptr_eq(&result.snapshot, &snapshot));
    assert!(result.effects.is_empty());
    let TransitionOutcome::Applied { result } = transition.outcome(&snapshot, Event::Emit).unwrap()
    else {
        panic!("command-only transitions must be applied");
    };
    assert_eq!(
        **result.into_commands().unwrap().commands.first().unwrap(),
        "command"
    );
}

#[test]
fn typed_guards_reject_lifecycle_events_before_reduction_and_validation() {
    struct Rejection(Rc<()>);

    let checks = Rc::new(Cell::new(0_u32));
    let reductions = checks.clone();
    let schemas = checks.clone();
    let transition: Transition<bool, ()> = Transition::new(move |_, (), enqueue| {
        reductions.set(reductions.get().saturating_add(1));
        enqueue.effect(|_| panic!("rejected events must not yield effects"));
        Ok(Update::replace(true))
    })
    .with_schemas(Schemas::default().with_events(Schema::new(
        SchemaMetadata::new("event"),
        move |(): &()| {
            schemas.set(schemas.get().saturating_add(1));
            Ok(SchemaCheck::Valid)
        },
    )))
    .with_validation(ValidationOptions::default());
    let snapshot = Rc::new(Snapshot::new(Rc::new(true)));
    let reason = Rc::new(());
    let reason_handle = reason.clone();
    let outcome = transition
        .outcome_with_guard(&snapshot, (), move |current, ()| {
            assert!(*current.context);
            Err(Rejection(reason_handle))
        })
        .unwrap();

    let TransitionOutcome::Rejected { reason: rejected } = outcome else {
        panic!("expected typed lifecycle rejection");
    };
    assert!(Rc::ptr_eq(&rejected.0, &reason));
    assert_eq!(checks.get(), 0);
    assert!(*snapshot.context);
}

#[test]
fn validation_failures_keep_the_original_error_details_and_sources() {
    let transition = Transition::new(|_: &Rc<i32>, event: &Event, enqueue| {
        enqueue.effect(|_| panic!("validation failure must not run effects"));
        if matches!(event, Event::InvalidCommand) {
            enqueue.emit(-1);
        }
        Ok(Update::replace(if matches!(event, Event::InvalidContext) {
            -1
        } else {
            1
        }))
    })
    .with_schemas(
        Schemas::default()
            .with_events(Schema::new(
                SchemaMetadata::new("events"),
                |event: &Event| {
                    Ok(if matches!(event, Event::Invalid) {
                        SchemaCheck::invalid(Vec::new())
                    } else {
                        SchemaCheck::Valid
                    })
                },
            ))
            .with_context(Schema::new(
                SchemaMetadata::new("context"),
                |context: &i32| {
                    Ok(if *context < 0 {
                        SchemaCheck::invalid(Vec::new())
                    } else {
                        SchemaCheck::Valid
                    })
                },
            ))
            .with_emitted(Schema::new(SchemaMetadata::new("commands"), |_: &i32| {
                Ok(SchemaCheck::invalid(Vec::new()))
            })),
    )
    .with_validation(ValidationOptions::default());
    let snapshot = Rc::new(Snapshot::new(Rc::new(0)));

    for (event, stage) in [
        (Event::Invalid, ValidationStage::Event),
        (Event::InvalidContext, ValidationStage::Context),
        (Event::InvalidCommand, ValidationStage::Emitted),
    ] {
        let TransitionOutcome::ValidationFailed { error } =
            transition.outcome(&snapshot, event).unwrap()
        else {
            panic!("expected a structured validation failure");
        };
        assert_eq!(error.dispatch_status(), None);
        assert_eq!(error.validation_error().unwrap().stage, stage);
        assert!(error.source().unwrap().is::<ValidationError>());
        assert_eq!(*snapshot.context, 0);
    }
}

#[test]
fn operational_failures_preserve_sources_and_remain_errors() {
    let nested: Store<(), ()> = Store::new(StoreConfig::new((), |_, (), _| {
        Err(StoreError::from_error(std::io::Error::other("failed")))
    }));
    let nested_reducer = nested.clone();
    let reducer: Transition<(), ()> = Transition::new(move |_, (), _| {
        nested_reducer.send(())?;
        Ok(Update::Keep)
    });
    let provider: Transition<(), ()> = Transition::new(|_, (), _| Ok(Update::Keep))
        .with_schemas(Schemas::default().with_events(Schema::new(
            SchemaMetadata::new("events"),
            move |(): &()| {
                nested.send(())?;
                Ok(SchemaCheck::Valid)
            },
        )))
        .with_validation(ValidationOptions::default());
    let snapshot = Rc::new(Snapshot::new(Rc::new(())));

    for transition in [reducer, provider] {
        let failure = transition.outcome(&snapshot, ()).unwrap_err();
        assert!(failure.validation_error().is_none());
        assert_eq!(failure.dispatch_status(), None);
        assert!(failure.source().unwrap().is::<std::io::Error>());
    }
}

#[test]
fn guards_can_reenter_the_same_transition_before_allowing_the_outer_event() {
    let transition: Transition<i32, i32> = Transition::new(|context: &Rc<i32>, event, _| {
        Ok(Update::replace(context.saturating_add(*event)))
    });
    let snapshot = Rc::new(Snapshot::new(Rc::new(1)));
    let guards = Rc::new(Cell::new(0_u32));
    let calls = guards.clone();
    let outcome = transition
        .outcome_with_guard(&snapshot, 2, |current, event| {
            calls.set(calls.get().saturating_add(1));
            assert_eq!(*current.context, 1);
            assert_eq!(*event, 2);
            let TransitionOutcome::Applied { result } = transition.outcome(&snapshot, 10).unwrap()
            else {
                panic!("expected nested proposal");
            };
            assert_eq!(*result.snapshot.context, 11);
            Ok::<_, ()>(())
        })
        .unwrap();
    let TransitionOutcome::Applied { result } = outcome else {
        panic!("expected outer proposal");
    };

    assert_eq!(guards.get(), 1);
    assert_eq!(*result.snapshot.context, 3);
    assert_eq!(*snapshot.context, 1);
}
