use std::{cell::Cell, rc::Rc};

use rstate::{Schema, SchemaCheck, Store};

use crate::validation::support::{Context, Event, config, schemas, trace};

#[test]
fn schema_declarations_are_inert_and_preserve_store_identity() {
    let trace = trace();
    let declared = schemas(&trace, Rc::new(Cell::new(0)));
    let store = Store::new(config(-1, &trace).with_schemas(declared));
    let initial = store.get_snapshot();

    assert!(trace.borrow().is_empty());
    let metadata = store.schemas().context().unwrap().metadata();
    assert_eq!(metadata.name, "counter");
    assert_eq!(metadata.vendor.as_deref(), Some("native-test"));

    store.send(Event::Keep).unwrap();
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    store.send(Event::Add(-1)).unwrap();
    assert_eq!(store.get_snapshot().context.count, -2);
    assert_eq!(*trace.borrow(), ["reduce:keep", "reduce:add", "effect:add"]);
}

#[test]
fn schemas_clone_and_debug_without_value_bounds_or_validation_calls() {
    let trace = trace();
    let declared = schemas(&trace, Rc::new(Cell::new(0)));
    let cloned = declared.clone();
    let check: Schema<Context> = cloned.context().unwrap().clone();

    let descriptor_display = format!("{check:?}");
    let set_display = format!("{cloned:?}");
    assert_ne!(descriptor_display, "");
    assert_ne!(set_display, "");
    assert_eq!(
        declared.context().unwrap().metadata(),
        cloned.context().unwrap().metadata()
    );
    assert!(trace.borrow().is_empty());

    assert!(matches!(
        check
            .check(&crate::validation::support::context(-1))
            .unwrap(),
        SchemaCheck::Invalid { .. }
    ));
    assert_eq!(*trace.borrow(), ["context:-1"]);
}
