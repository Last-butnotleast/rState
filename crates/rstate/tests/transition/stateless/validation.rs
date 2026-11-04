use std::{
    cell::{Cell, RefCell},
    error::Error,
    rc::Rc,
};

use rstate::{
    Schema, SchemaCheck, SchemaMetadata, Schemas, Snapshot, Store, StoreConfig, StoreError,
    Transition, Update, ValidationOptions, ValidationStage,
};

enum Event {
    Repair,
    Internal,
    Replace(i32),
    EmitInvalid,
    Invalid,
}

type Trace = Rc<RefCell<Vec<&'static str>>>;

fn definition(trace: &Trace, runs: &Rc<Cell<u32>>) -> Transition<i32, Event, i32> {
    let reduced = trace.clone();
    let effect_runs = runs.clone();
    let event_trace = trace.clone();
    let context_trace = trace.clone();
    let emitted_trace = trace.clone();
    Transition::new(move |_: &Rc<i32>, event: &Event, enqueue| {
        reduced.borrow_mut().push("reduce");
        let runs = effect_runs.clone();
        enqueue.effect(move |_| {
            runs.set(runs.get().saturating_add(1));
            Ok(())
        });
        let next = match event {
            Event::Repair => {
                enqueue.trigger(Event::Internal);
                -1
            }
            Event::Internal => {
                enqueue.emit(1);
                1
            }
            Event::Replace(value) => *value,
            Event::EmitInvalid => {
                enqueue.emit(-1);
                0
            }
            Event::Invalid => 0,
        };
        Ok(Update::replace(next))
    })
    .with_schemas(
        Schemas::default()
            .with_events(Schema::new(
                SchemaMetadata::new("events"),
                move |event: &Event| {
                    event_trace.borrow_mut().push("event");
                    Ok(if matches!(event, Event::Invalid | Event::Internal) {
                        SchemaCheck::invalid(Vec::new())
                    } else {
                        SchemaCheck::Valid
                    })
                },
            ))
            .with_context(Schema::new(
                SchemaMetadata::new("context"),
                move |context: &i32| {
                    context_trace.borrow_mut().push("context");
                    Ok(if *context < 0 {
                        SchemaCheck::invalid(Vec::new())
                    } else {
                        SchemaCheck::Valid
                    })
                },
            ))
            .with_emitted(Schema::new(
                SchemaMetadata::new("emitted"),
                move |message: &i32| {
                    emitted_trace.borrow_mut().push("emitted");
                    Ok(if *message < 0 {
                        SchemaCheck::invalid(Vec::new())
                    } else {
                        SchemaCheck::Valid
                    })
                },
            )),
    )
}

#[test]
fn schemas_are_inert_until_validation_is_enabled() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let runs = Rc::new(Cell::new(0));
    let transition = definition(&trace, &runs);
    assert_eq!(
        transition.schemas().events().unwrap().metadata().name,
        "events"
    );
    let snapshot = Rc::new(Snapshot::new(Rc::new(0)));

    let result = transition.transition(&snapshot, Event::Invalid).unwrap();
    assert!(result.allowed);
    assert_eq!(*trace.borrow(), ["reduce"]);
    assert_eq!(runs.get(), 0);
}

#[test]
fn opt_in_validation_checks_final_context_and_bypasses_internal_event_checks() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let runs = Rc::new(Cell::new(0));
    let transition = definition(&trace, &runs).with_validation(ValidationOptions::default());
    let snapshot = Rc::new(Snapshot::new(Rc::new(-10)));

    let result = transition.transition(&snapshot, Event::Repair).unwrap();
    assert_eq!(*result.snapshot.context, 1);
    assert_eq!(*snapshot.context, -10);
    assert_eq!(
        *trace.borrow(),
        ["event", "reduce", "reduce", "context", "emitted"]
    );
    assert_eq!(runs.get(), 0);
}

#[test]
fn validation_failures_discard_proposed_state_and_all_effects() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let runs = Rc::new(Cell::new(0));
    let transition = definition(&trace, &runs).with_validation(ValidationOptions::default());
    let snapshot = Rc::new(Snapshot::new(Rc::new(10)));

    for (event, stage, expected) in [
        (Event::Invalid, ValidationStage::Event, vec!["event"]),
        (
            Event::Replace(-1),
            ValidationStage::Context,
            vec!["event", "reduce", "context"],
        ),
        (
            Event::EmitInvalid,
            ValidationStage::Emitted,
            vec!["event", "reduce", "context", "emitted"],
        ),
    ] {
        trace.borrow_mut().clear();
        let failure = transition.transition(&snapshot, event).unwrap_err();
        assert_eq!(failure.validation_error().unwrap().stage, stage);
        assert_eq!(failure.dispatch_status(), None);
        assert_eq!(*trace.borrow(), expected);
        assert_eq!(*snapshot.context, 10);
        assert_eq!(runs.get(), 0);
    }
}

#[test]
fn pure_errors_preserve_application_sources_and_clear_nested_dispatch_status() {
    let nested = Store::<i32, ()>::new(StoreConfig::new(0, |_, (), _| {
        Err(StoreError::from_error(std::io::Error::other(
            "nested failure",
        )))
    }));
    assert!(nested.send(()).unwrap_err().dispatch_status().is_some());
    let callback_store = nested.clone();
    let reducer: Transition<i32, ()> = Transition::new(move |_, (), _| {
        callback_store.send(())?;
        Ok(Update::Keep)
    });
    let validated = nested;
    let schema: Transition<i32, ()> = Transition::new(|_, (), _| Ok(Update::Keep))
        .with_schemas(Schemas::default().with_events(Schema::new(
            SchemaMetadata::new("events"),
            move |(): &()| {
                validated.send(())?;
                Ok(SchemaCheck::Valid)
            },
        )))
        .with_validation(ValidationOptions::default());
    let snapshot = Rc::new(Snapshot::new(Rc::new(0)));

    for transition in [reducer, schema] {
        let failure = transition.transition(&snapshot, ()).unwrap_err();
        assert_eq!(failure.dispatch_status(), None);
        assert!(failure.validation_error().is_none());
        let cause = failure.source().unwrap();
        assert!(cause.downcast_ref::<std::io::Error>().is_some());
        assert_eq!(cause.to_string(), "nested failure");
    }
}
