use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{
    Inspection, Schema, SchemaCheck, SchemaMetadata, Schemas, Store, StoreConfig, Update,
    ValidationOptions, ValidationStage,
};

use crate::validation::support::{Event, Message, config, context, schemas, trace};

#[test]
fn validation_reduction_and_inspection_borrow_the_same_non_clone_event() {
    let pointers = Rc::new(RefCell::new(Vec::new()));
    let checked = pointers.clone();
    let reduced = pointers.clone();
    let inspected = pointers.clone();
    let schema = Schema::new(SchemaMetadata::new("events"), move |event: &Event| {
        checked.borrow_mut().push(std::ptr::from_ref(event));
        Ok(SchemaCheck::Valid)
    });
    let store = Store::<_, _, Message>::new_validated(
        StoreConfig::new(context(0), move |context, event, _| {
            reduced.borrow_mut().push(std::ptr::from_ref(event));
            Ok(Update::Replace(context.clone()))
        })
        .with_schemas(Schemas::default().with_events(schema)),
        ValidationOptions::default(),
    )
    .unwrap();
    let _observer = store
        .inspect(move |record| {
            if let Inspection::Transition { event, .. } = record {
                inspected.borrow_mut().push(std::ptr::from_ref(event));
            }
            Ok(())
        })
        .unwrap();
    let initial = store.get_snapshot();
    store.send(Event::Keep).unwrap();

    let recorded = pointers.borrow();
    assert_eq!(recorded.len(), 3);
    let first = recorded.first().unwrap();
    assert!(recorded.iter().all(|pointer| pointer == first));
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
}

#[test]
fn invalid_outer_events_do_not_run_reducers_or_change_history_and_observers() {
    let trace = trace();
    let store = Store::new_validated(
        config(0, &trace)
            .with_history(4)
            .with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default(),
    )
    .unwrap();
    let notifications = Rc::new(Cell::new(0_u32));
    let observed = notifications.clone();
    let _subscription = store.subscribe(move |_| {
        observed.set(observed.get().saturating_add(1));
        Ok(())
    });
    let inspected = notifications.clone();
    let _inspection = store
        .inspect(move |record| {
            if matches!(record, Inspection::Transition { .. }) {
                inspected.set(inspected.get().saturating_add(10));
            }
            Ok(())
        })
        .unwrap();
    let original = store.get_snapshot();
    trace.borrow_mut().clear();

    let failure = store.send(Event::Add(-1)).unwrap_err();
    assert_eq!(
        failure.validation_error().unwrap().stage,
        ValidationStage::Event
    );
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);
    assert_eq!(notifications.get(), 0);
    assert_eq!(*trace.borrow(), ["event:add"]);
}

#[test]
fn context_checks_only_the_final_macrostep_and_dry_runs_leave_history_untouched() {
    let trace = trace();
    let store = Store::new_validated(
        config(0, &trace)
            .with_history(4)
            .with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default(),
    )
    .unwrap();
    let original = store.get_snapshot();
    trace.borrow_mut().clear();

    let rejected = store.send(Event::Break).unwrap_err();
    assert_eq!(
        rejected.validation_error().unwrap().stage,
        ValidationStage::Context
    );
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);
    assert_eq!(
        *trace.borrow(),
        ["event:break", "reduce:break", "context:-1"]
    );

    trace.borrow_mut().clear();
    let predicted = store.transition(original.clone(), Event::Repair).unwrap();
    assert_eq!(predicted.snapshot.context.count, 1);
    assert_eq!(predicted.snapshot.history_status().past, 1);
    assert!(predicted.effects.is_empty());
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);
    assert_eq!(
        *trace.borrow(),
        [
            "event:repair",
            "reduce:repair",
            "reduce:internal",
            "context:1"
        ]
    );

    trace.borrow_mut().clear();
    assert!(store.can(Event::Repair).unwrap());
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
    store.send(Event::Repair).unwrap();
    assert_eq!(store.get_snapshot().context.count, 1);
    assert_eq!(store.history_status().past, 1);
}

#[test]
fn internally_triggered_event_payloads_bypass_outer_event_validation() {
    let trace = trace();
    let store = Store::new_validated(
        config(0, &trace).with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default(),
    )
    .unwrap();
    trace.borrow_mut().clear();

    assert!(store.send(Event::Internal(5)).is_err());
    assert_eq!(store.get_snapshot().context.count, 0);
    trace.borrow_mut().clear();
    store.send(Event::Start).unwrap();
    assert_eq!(store.get_snapshot().context.count, 5);
    assert_eq!(
        *trace.borrow(),
        [
            "event:start",
            "reduce:start",
            "reduce:internal",
            "context:5"
        ]
    );
}

#[test]
fn emitted_checks_finish_before_any_mixed_effect_or_emission_runs() {
    let trace = trace();
    let store = Store::new_validated(
        config(0, &trace)
            .with_history(4)
            .with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default(),
    )
    .unwrap();
    let emitted = trace.clone();
    let _listener = store.on_any(move |message| {
        match message.as_ref() {
            Message::Value(value) => emitted.borrow_mut().push(format!("emit:{value}")),
            Message::Unknown => emitted.borrow_mut().push("emit:unknown".into()),
        }
        Ok(())
    });
    let initial = store.get_snapshot();
    trace.borrow_mut().clear();

    let failure = store.send(Event::Emit(-1)).unwrap_err();
    assert_eq!(
        failure.validation_error().unwrap().stage,
        ValidationStage::Emitted
    );
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);
    assert_eq!(
        *trace.borrow(),
        ["event:emit", "reduce:emit", "context:0", "emitted:-1"]
    );

    trace.borrow_mut().clear();
    store.send(Event::Emit(2)).unwrap();
    assert_eq!(store.history_status().past, 1);
    assert_eq!(
        *trace.borrow(),
        [
            "event:emit",
            "reduce:emit",
            "context:0",
            "emitted:2",
            "effect:before",
            "emit:2",
            "effect:after",
        ]
    );
}
