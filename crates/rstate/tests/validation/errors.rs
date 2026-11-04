use std::{cell::Cell, error::Error, rc::Rc};

use rstate::{
    PathSegment, Store, UnknownPolicy, ValidationError, ValidationOptions, ValidationReason,
};

use super::support::{Event, config, schemas, trace};

#[test]
fn can_suppresses_only_validation_errors_and_does_not_execute_effects() {
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

    for event in [
        Event::Add(-1),
        Event::Break,
        Event::Emit(-1),
        Event::Unknown,
        Event::UnknownEmission,
    ] {
        assert!(!store.can(event).unwrap());
    }
    let provider = store.can(Event::ProviderFailure).unwrap_err();
    assert!(provider.validation_error().is_none());
    assert_eq!(provider.to_string(), "provider failed");
    assert!(provider.source().unwrap().is::<std::io::Error>());
    let reducer = store.can(Event::ReducerFailure).unwrap_err();
    assert!(reducer.validation_error().is_none());
    assert_eq!(reducer.to_string(), "reducer failed");
    assert!(Rc::ptr_eq(&original, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);
    assert!(
        !trace
            .borrow()
            .iter()
            .any(|entry| entry.starts_with("effect:"))
    );
}

#[test]
fn validation_errors_expose_schema_names_structured_paths_and_error_sources() {
    fn assert_error_traits<T: Error + Send + Sync + 'static>() {}

    assert_error_traits::<ValidationError>();
    let trace = trace();
    let store = Store::new_validated(
        config(0, &trace).with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default(),
    )
    .unwrap();
    let failure = store.send(Event::Emit(-1)).unwrap_err();
    let error = failure.validation_error().unwrap();
    assert_eq!(error.reason, ValidationReason::InvalidEmitted);
    assert_eq!(error.schema.name, "messages");
    assert_eq!(error.name.as_deref(), Some("value"));
    assert_eq!(error.issues.len(), 1);
    let issue = error.issues.first().unwrap();
    assert_eq!(issue.message, "invalid value");
    assert_eq!(
        issue.path,
        [
            PathSegment::Field("rows".into()),
            PathSegment::Index(0),
            PathSegment::Field("value".into()),
        ]
    );
    assert_eq!(failure.to_string(), "Invalid emitted event \"value\"");
    assert_eq!(
        failure.source().unwrap().downcast_ref::<ValidationError>(),
        Some(error)
    );

    let event = store.send(Event::Add(-1)).unwrap_err();
    assert_eq!(
        event.validation_error().unwrap().reason,
        ValidationReason::InvalidEvent
    );
    let unknown = store.send(Event::Unknown).unwrap_err();
    assert_eq!(
        unknown.validation_error().unwrap().reason,
        ValidationReason::UnknownEvent
    );
    assert_eq!(unknown.to_string(), "Unknown event \"unknown\"");
    let emitted = store.send(Event::UnknownEmission).unwrap_err();
    assert_eq!(
        emitted.validation_error().unwrap().reason,
        ValidationReason::UnknownEmitted
    );
}

#[test]
fn validation_options_disable_only_the_selected_area_or_unknown_policy() {
    let trace = trace();
    let events_disabled = Store::new_validated(
        config(4, &trace).with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default().with_events(false),
    )
    .unwrap();
    trace.borrow_mut().clear();
    events_disabled.send(Event::Add(-1)).unwrap();
    assert_eq!(events_disabled.get_snapshot().context.count, 3);
    assert!(
        !trace
            .borrow()
            .iter()
            .any(|entry| entry.starts_with("event:"))
    );

    let context_disabled = Store::new_validated(
        config(-1, &trace).with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default().with_context(false),
    )
    .unwrap();
    trace.borrow_mut().clear();
    context_disabled.send(Event::Break).unwrap();
    assert_eq!(context_disabled.get_snapshot().context.count, -1);
    assert!(
        !trace
            .borrow()
            .iter()
            .any(|entry| entry.starts_with("context:"))
    );

    let emitted_disabled = Store::new_validated(
        config(0, &trace).with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default().with_emitted(false),
    )
    .unwrap();
    trace.borrow_mut().clear();
    emitted_disabled.send(Event::Emit(-1)).unwrap();
    assert_eq!(
        *trace.borrow(),
        [
            "event:emit",
            "reduce:emit",
            "context:0",
            "effect:before",
            "effect:after"
        ]
    );

    let unknown_allowed = Store::new_validated(
        config(0, &trace).with_schemas(schemas(&trace, Rc::new(Cell::new(0)))),
        ValidationOptions::default()
            .with_unknown_events(UnknownPolicy::Ignore)
            .with_unknown_emitted(UnknownPolicy::Ignore),
    )
    .unwrap();
    unknown_allowed.send(Event::Unknown).unwrap();
    unknown_allowed.send(Event::UnknownEmission).unwrap();
    assert!(!unknown_allowed.can(Event::Unknown).unwrap());
}
