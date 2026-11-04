use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui_kit::TestAppContext;
use rstate::{DispatchStatus, StoreError};
use rstate_gpui::{CommandPolicy, CommandScope};

use super::support::unexpected;
use crate::support::{Event, counter};

#[gpui_kit::test]
fn completion_dispatch_is_synchronous_and_delivery_errors_keep_commit_status(cx: &TestAppContext) {
    let store = counter();
    let weak = store.downgrade();
    let _guard = store
        .subscribe(move |snapshot| {
            if *snapshot.context == 1 {
                weak.upgrade().unwrap().send(Event::Set(2))?;
                return Err(StoreError::new("outer delivery failed"));
            }
            Ok(())
        })
        .scoped();
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let errors = Rc::new(RefCell::new(Vec::new()));
    let seen = errors.clone();
    let weak = store.downgrade();
    channel
        .submit(
            |_| async { Event::Set(1) },
            move |event, _| weak.upgrade().unwrap().send(event),
            move |error, _| seen.borrow_mut().push(error),
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(*store.get_snapshot().context, 2);
    let errors = errors.borrow();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].to_string(), "outer delivery failed");
    assert_eq!(errors[0].dispatch_status(), Some(DispatchStatus::Committed));
}

#[gpui_kit::test]
fn error_handler_can_close_scope_before_next_serial_command(cx: &TestAppContext) {
    let scope = Rc::new(cx.update(|cx| CommandScope::new(cx)));
    let channel = scope.channel(CommandPolicy::Serial);
    let owner = Rc::downgrade(&scope);
    let errors = Rc::new(Cell::new(0_u32));
    let seen = errors.clone();
    channel
        .submit(
            |_| async {},
            |(), _| Err(StoreError::new("expected delivery failure")),
            move |_, _| {
                seen.set(seen.get().saturating_add(1));
                owner.upgrade().unwrap().close();
            },
        )
        .unwrap();
    channel
        .submit(
            |_| -> std::future::Ready<()> {
                panic!("factory started after error handler closed scope");
            },
            |(), _| Ok(()),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(errors.get(), 1);
    assert!(scope.is_closed());
}
