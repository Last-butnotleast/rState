use std::{cell::Cell, rc::Rc};

use rstate::{Store, StoreError, ValidationOptions, ValidationStage};

use super::support::{Event, config, context, schemas, trace};

#[test]
fn rejected_control_restoration_keeps_context_history_and_notifications_unchanged() {
    let trace = trace();
    let minimum = Rc::new(Cell::new(0));
    let store = Store::new_validated(
        config(2, &trace)
            .with_history(4)
            .with_schemas(schemas(&trace, minimum.clone())),
        ValidationOptions::default(),
    )
    .unwrap();
    store.send(Event::Add(1)).unwrap();
    store.send(Event::Add(1)).unwrap();
    let notifications = Rc::new(Cell::new(0_u32));
    let observed = notifications.clone();
    let _listener = store.subscribe(move |_| {
        observed.set(observed.get().saturating_add(1));
        Ok(())
    });
    let initial_status = store.history_status();
    let current = store.get_snapshot();
    minimum.set(4);
    trace.borrow_mut().clear();

    assert!(store.can_undo());
    let failure = store.undo().unwrap_err();
    assert_eq!(
        failure.validation_error().unwrap().stage,
        ValidationStage::Context
    );
    assert!(Rc::ptr_eq(&current, &store.get_snapshot()));
    assert_eq!(store.history_status(), initial_status);
    assert_eq!(notifications.get(), 0);
    assert_eq!(*trace.borrow(), ["context:3"]);

    minimum.set(0);
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 3);
    assert_eq!(notifications.get(), 1);
    let undone = store.get_snapshot();
    let undone_status = store.history_status();
    minimum.set(5);
    assert!(store.can_redo());
    assert!(store.redo().is_err());
    assert!(Rc::ptr_eq(&undone, &store.get_snapshot()));
    assert_eq!(store.history_status(), undone_status);
    assert_eq!(notifications.get(), 1);

    store.reset_with(|_, _| Ok(Rc::new(context(6)))).unwrap();
    assert_eq!(store.get_snapshot().context.count, 6);
    assert_eq!(notifications.get(), 2);
    let reset = store.get_snapshot();
    let reset_status = store.history_status();
    assert!(store.reset().is_err());
    assert!(Rc::ptr_eq(&reset, &store.get_snapshot()));
    assert_eq!(store.history_status(), reset_status);
    assert_eq!(notifications.get(), 2);

    let mapper_failure = store
        .reset_with(|_, _| Err(StoreError::new("mapper failed")))
        .unwrap_err();
    assert!(mapper_failure.validation_error().is_none());
    assert!(Rc::ptr_eq(&reset, &store.get_snapshot()));
}
