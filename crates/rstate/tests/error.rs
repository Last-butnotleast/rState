use std::{error::Error, io, rc::Rc};

use rstate::{
    SchemaMetadata, Store, StoreConfig, StoreError, ValidationError, ValidationIssue,
    ValidationReason, ValidationStage,
};

#[derive(Debug, thiserror::Error)]
#[error("application operation failed")]
struct ApplicationFailure(#[source] io::Error);

const fn assert_shareable_error<T: Error + Send + Sync + 'static>() {}

fn propagate_store_failure() -> Result<(), Box<dyn Error + Send + Sync>> {
    let store: Store<Rc<()>, ()> = Store::new(StoreConfig::new(Rc::new(()), |_, (), _| {
        Err(StoreError::from_error(io::Error::other(
            "application failure",
        )))
    }));

    store.send(())?;
    Ok(())
}

#[test]
fn store_errors_propagate_into_shareable_application_error_containers() {
    assert_shareable_error::<StoreError>();

    let error = propagate_store_failure().unwrap_err();
    let store_error = error.downcast_ref::<StoreError>().unwrap();
    let original = store_error
        .source()
        .unwrap()
        .downcast_ref::<io::Error>()
        .unwrap();

    assert_eq!(original.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "application failure");
}

#[test]
fn external_error_exposes_the_original_concrete_source() {
    let error = StoreError::from_error(io::Error::new(io::ErrorKind::NotFound, "missing state"));
    let source = error.source().unwrap();
    let original = source.downcast_ref::<io::Error>().unwrap();

    assert_eq!(original.kind(), io::ErrorKind::NotFound);
    assert_eq!(error.to_string(), "missing state");
    assert!(source.source().is_none());
    assert_eq!(error.dispatch_status(), None);
}

#[test]
fn external_error_preserves_the_complete_application_source_chain() {
    let error = StoreError::from_error(ApplicationFailure(io::Error::other("storage unavailable")));
    let application = error.source().unwrap();
    let storage = application.source().unwrap();

    assert!(application.downcast_ref::<ApplicationFailure>().is_some());
    assert_eq!(error.to_string(), "application operation failed");
    assert_eq!(application.to_string(), "application operation failed");
    assert_eq!(
        storage.downcast_ref::<io::Error>().unwrap().to_string(),
        "storage unavailable"
    );
    assert!(storage.source().is_none());
}

#[test]
fn callback_messages_remain_errors_without_a_source() {
    let error = StoreError::new("callback rejected the event");

    assert_eq!(error.to_string(), "callback rejected the event");
    assert!(error.source().is_none());
    assert_eq!(error.dispatch_status(), None);
}

#[test]
fn structured_validation_errors_can_be_constructed_by_downstream_providers() {
    let validation = ValidationError::new(
        ValidationStage::Event,
        ValidationReason::InvalidEvent,
        SchemaMetadata::new("event schema").with_vendor("application"),
    )
    .named("submit")
    .with_issues(vec![ValidationIssue::new("missing title")]);
    let error = StoreError::from(validation.clone());

    assert_eq!(error.validation_error(), Some(&validation));
    assert_eq!(error.dispatch_status(), None);
    assert_eq!(error.to_string(), "Invalid event \"submit\"");
    assert_eq!(
        error.source().unwrap().downcast_ref::<ValidationError>(),
        Some(&validation)
    );
    assert_eq!(validation.schema.vendor.as_deref(), Some("application"));
}
