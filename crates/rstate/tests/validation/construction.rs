use std::{cell::Cell, rc::Rc};

use rstate::{Store, StoreConfig, StoreFactory, Update, ValidationOptions, ValidationStage};

use crate::validation::support::{Context, Event, Message, config, context, schemas, trace};

#[test]
fn initial_validation_is_fallible_and_keeps_the_original_context_on_success() {
    let trace = trace();
    let declared = schemas(&trace, Rc::new(Cell::new(0)));
    let invalid = Store::new_validated(
        config(-1, &trace).with_schemas(declared.clone()),
        ValidationOptions::default(),
    )
    .err()
    .unwrap();
    assert_eq!(
        invalid.validation_error().unwrap().stage,
        ValidationStage::InitialContext
    );
    assert_eq!(*trace.borrow(), ["context:-1"]);

    trace.borrow_mut().clear();
    let store = Store::new_validated(
        config(2, &trace).with_schemas(declared),
        ValidationOptions::default(),
    )
    .unwrap();
    let initial = store.get_initial_snapshot();
    let current = store.get_snapshot();
    assert!(Rc::ptr_eq(&initial, &current));
    assert!(Rc::ptr_eq(&initial.context, &current.context));
    assert_eq!(*trace.borrow(), ["context:2"]);

    trace.borrow_mut().clear();
    store.send(Event::Keep).unwrap();
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    assert_eq!(*trace.borrow(), ["event:keep", "reduce:keep", "context:2"]);
}

#[test]
fn factories_share_schema_definitions_without_clone_bounds_or_implicit_validation() {
    struct Input(i32);

    let trace = trace();
    let factory: StoreFactory<Input, Context, Event, Message> = StoreFactory::new(
        |input: Input| Ok(context(input.0)),
        |_, _, _| Ok(Update::Keep),
    )
    .with_schemas(schemas(&trace, Rc::new(Cell::new(0))));
    let cloned = factory.clone();
    let ordinary = factory.create(Input(-2)).unwrap();
    assert_eq!(ordinary.get_snapshot().context.count, -2);
    assert!(trace.borrow().is_empty());

    assert!(
        cloned
            .create_validated(Input(-2), ValidationOptions::default())
            .is_err()
    );
    let first = cloned
        .create_validated(Input(1), ValidationOptions::default())
        .unwrap();
    let second = cloned
        .create_validated(Input(3), ValidationOptions::default())
        .unwrap();
    assert_eq!(first.get_snapshot().context.count, 1);
    assert_eq!(second.get_snapshot().context.count, 3);
    assert!(!Rc::ptr_eq(
        &first.get_snapshot().context,
        &second.get_snapshot().context
    ));
    assert_eq!(*trace.borrow(), ["context:-2", "context:1", "context:3"]);

    let metadata_only = Store::new(StoreConfig::<_, Event, Message>::new(
        context(-1),
        |_, _, _| Ok(Update::Keep),
    ));
    assert!(metadata_only.schemas().context().is_none());
}
