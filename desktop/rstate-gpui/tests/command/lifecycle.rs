use std::{cell::RefCell, rc::Rc};

use gpui_kit::TestAppContext;
use rstate_gpui::{CommandPolicy, CommandScope, CommandSubmitError};

use super::support::{Gate, unexpected};
use crate::support::{Event, counter, observe, view};

#[gpui_kit::test]
fn scope_invalidation_is_reusable_and_close_rejects_submissions(cx: &TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let gate = Gate::default();
    let wait = gate.clone();
    let ticket = channel
        .submit(
            move |_| async move { wait.wait().await },
            |(), _| panic!("invalidated output delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(scope.invalidate(), 1);
    assert!(!ticket.is_pending());
    gate.release();
    let output = Rc::new(RefCell::new(None));
    let delivered = output.clone();
    let next = channel
        .submit(
            |_| async { 4 },
            move |value, _| {
                delivered.replace(Some(value));
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    assert!(next.id() > ticket.id());
    cx.run_until_parked();
    assert_eq!(*output.borrow(), Some(4));
    assert_eq!(scope.close(), 0);
    assert!(scope.is_closed());
    assert!(channel.is_closed());
    assert_eq!(
        channel
            .submit(|_| async {}, |(), _| Ok(()), unexpected)
            .unwrap_err(),
        CommandSubmitError::Closed
    );
}

#[gpui_kit::test]
fn dropping_scope_cancels_owned_futures_without_channel_cycles(cx: &TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    let gate = Gate::default();
    let ticket = channel
        .submit(
            move |_| async move {
                gate.wait().await;
                drop(capture);
            },
            |(), _| panic!("closed output delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert!(weak.upgrade().is_some());
    drop(scope);
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
    assert!(!ticket.is_pending());
    assert!(channel.is_closed());
    assert_eq!(
        channel
            .submit(|_| async {}, |(), _| Ok(()), unexpected)
            .unwrap_err(),
        CommandSubmitError::Closed
    );
}

#[gpui_kit::test]
fn manager_owned_work_survives_one_consuming_view(cx: &mut TestAppContext) {
    let store = counter();
    let first = view(cx, &store, 1);
    let second = view(cx, &store, 1);
    let count = observe(cx, &second);
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let gate = Gate::default();
    let wait = gate.clone();
    let weak = store.downgrade();
    channel
        .submit(
            move |_| async move {
                wait.wait().await;
                Event::Set(7)
            },
            move |event, _| weak.upgrade().unwrap().send(event),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    drop(first);
    gate.release();
    cx.run_until_parked();
    assert_eq!(second.read_with(cx, |view, _| *view.values[0].get()), 7);
    assert_eq!(count.get(), 1);
}

#[gpui_kit::test]
fn invalidating_one_channel_preserves_an_independent_operation(cx: &TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let first = scope.channel(CommandPolicy::Latest);
    let second = scope.channel(CommandPolicy::Concurrent);
    let cancelled = first
        .submit(
            |_| async {},
            |(), _| panic!("invalidated operation delivered"),
            unexpected,
        )
        .unwrap();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    let retained = second
        .submit(
            |_| async { 8 },
            move |value, _| {
                seen.borrow_mut().push(value);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    assert_eq!(first.invalidate(), 1);
    assert!(!cancelled.is_pending());
    assert!(retained.is_pending());
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[8]);
}

#[gpui_kit::test]
fn dropping_scope_cancels_a_serial_request_already_draining(cx: &TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Serial);
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    let gate = Gate::default();
    let ticket = channel
        .submit(
            move |_| async move {
                gate.wait().await;
                drop(capture);
            },
            |(), _| panic!("cancelled serial request delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert!(ticket.cancel());
    cx.run_until_parked();
    assert!(weak.upgrade().is_some());
    drop(scope);
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
    assert!(channel.is_closed());
}

#[gpui_kit::test]
fn app_shutdown_cancels_queued_commands_before_app_release(cx: &TestAppContext) {
    let app = cx.new_app();
    let scope = app.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    let ticket = channel
        .submit(
            move |_| -> std::future::Ready<()> {
                drop(capture);
                panic!("shutdown command started")
            },
            |(), _| panic!("shutdown output delivered"),
            unexpected,
        )
        .unwrap();
    app.quit();
    assert!(scope.is_closed());
    assert!(channel.is_closed());
    assert!(!ticket.is_pending());
    assert!(weak.upgrade().is_none());
    drop(app);
    cx.run_until_parked();
    assert_eq!(
        channel
            .submit(|_| async {}, |(), _| Ok(()), unexpected)
            .unwrap_err(),
        CommandSubmitError::Closed
    );
}

#[gpui_kit::test]
fn app_shutdown_cancels_running_commands_and_suppresses_late_delivery(cx: &TestAppContext) {
    let app = cx.new_app();
    let scope = app.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    let gate = Gate::default();
    let wait = gate.clone();
    let ticket = channel
        .submit(
            move |_| async move {
                wait.wait().await;
                drop(capture);
            },
            |(), _| panic!("shutdown output delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert!(weak.upgrade().is_some());
    app.quit();
    assert!(scope.is_closed());
    assert!(!ticket.is_pending());
    drop(app);
    gate.release();
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
}
