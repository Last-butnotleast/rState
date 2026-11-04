use std::{cell::RefCell, rc::Rc};

use rstate::{DispatchStatus, StoreError};
use rstate_gpui::{dispatch, handle_result};

use crate::support::{Event, counter};

#[test]
fn dispatch_routes_reporting_call_error_once_without_retrying() {
    let store = counter();
    let errors = RefCell::new(Vec::new());
    let report = |error: StoreError| {
        errors
            .borrow_mut()
            .push((error.to_string(), error.dispatch_status()));
    };
    assert!(!dispatch(&store, Event::Fail, report));
    assert_eq!(*store.get_snapshot().context, 0);
    assert_eq!(
        &*errors.borrow(),
        &[("reducer failed".into(), Some(DispatchStatus::NotCommitted))]
    );
    errors.borrow_mut().clear();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let seen = calls.clone();
    let _guard = store
        .subscribe(move |snapshot| {
            seen.borrow_mut().push(*snapshot.context);
            Err(StoreError::new("delivery failed"))
        })
        .scoped();
    assert!(!dispatch(&store, Event::Set(4), report));
    assert_eq!(*store.get_snapshot().context, 4);
    assert_eq!(&*calls.borrow(), &[4]);
    assert_eq!(
        &*errors.borrow(),
        &[("delivery failed".into(), Some(DispatchStatus::Committed))]
    );
}

#[test]
fn successful_dispatch_and_controls_do_not_report_an_error() {
    let store = counter();
    let errors = RefCell::new(Vec::new());
    let report = |error: StoreError| errors.borrow_mut().push(error);
    assert!(dispatch(&store, Event::Set(1), report));
    assert!(handle_result(store.undo(), report));
    assert_eq!(*store.get_snapshot().context, 0);
    assert!(errors.borrow().is_empty());
}

#[test]
fn error_routing_preserves_nested_commits_and_reporting_status() {
    let store = counter();
    let weak = store.downgrade();
    let _guard = store
        .subscribe(move |snapshot| {
            if *snapshot.context == 1 {
                weak.upgrade().unwrap().send(Event::Set(2))?;
                return Err(StoreError::new("outer notification failed"));
            }
            Ok(())
        })
        .scoped();
    let mut reported = None;
    assert!(!dispatch(&store, Event::Set(1), |error| reported = Some(error)));
    let error = reported.unwrap();
    assert_eq!(error.dispatch_status(), Some(DispatchStatus::Committed));
    assert_eq!(*store.get_snapshot().context, 2);
    assert_eq!(store.history_status().past, 2);
}
