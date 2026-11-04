use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rstate_gpui::{CommandPolicy, CommandScope};

use super::support::{BackgroundGate, Gate, OnDrop, unexpected};

#[gpui::test]
fn cancelled_factory_captures_drop_outside_borrows_and_can_submit(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let values = Rc::new(RefCell::new(Vec::new()));
    let reenter = channel.clone();
    let delivered = values.clone();
    let capture = OnDrop(Some(Box::new(move || {
        reenter
            .submit(
                |_| async { 2 },
                move |value, _| {
                    delivered.borrow_mut().push(value);
                    Ok(())
                },
                unexpected,
            )
            .unwrap();
    })));
    channel
        .submit(
            move |_| {
                drop(capture);
                async {}
            },
            |(), _| panic!("invalidated factory ran"),
            unexpected,
        )
        .unwrap();
    assert_eq!(channel.invalidate(), 1);
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[2]);
}

#[gpui::test]
fn latest_reentrant_replacement_preserves_newest_request(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Latest);
    let values = Rc::new(RefCell::new(Vec::new()));
    let reenter = channel.clone();
    let delivered = values.clone();
    let capture = OnDrop(Some(Box::new(move || {
        reenter
            .submit(
                |_| async { 3 },
                move |value, _| {
                    delivered.borrow_mut().push(value);
                    Ok(())
                },
                unexpected,
            )
            .unwrap();
    })));
    channel
        .submit(
            move |_| {
                drop(capture);
                async {}
            },
            |(), _| panic!("replaced factory ran"),
            unexpected,
        )
        .unwrap();
    let superseded = channel
        .submit(
            |_| async { 2 },
            |_, _| panic!("older reentrant replacement delivered"),
            unexpected,
        )
        .unwrap();
    assert!(!superseded.is_pending());
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[3]);
}

#[gpui::test]
fn invalidating_serial_factory_drains_already_submitted_background_work(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Serial);
    let gate = BackgroundGate::default();
    let wait = gate.clone();
    let reenter = channel.clone();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let started = trace.clone();
    channel
        .submit(
            move |cx| {
                started.borrow_mut().push(1);
                let task = cx.background_executor().spawn(async move {
                    wait.wait().await;
                });
                assert_eq!(reenter.invalidate(), 1);
                let started = started.clone();
                reenter
                    .submit(
                        move |_| {
                            started.borrow_mut().push(2);
                            async {}
                        },
                        |(), _| Ok(()),
                        unexpected,
                    )
                    .unwrap();
                task
            },
            |(), _| panic!("invalidated serial output delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(&*trace.borrow(), &[1]);
    gate.release();
    cx.run_until_parked();
    assert_eq!(&*trace.borrow(), &[1, 2]);
}

#[gpui::test]
fn factory_can_close_its_scope_and_release_future_captures(cx: &mut TestAppContext) {
    let scope = Rc::new(RefCell::new(Some(cx.update(|cx| CommandScope::new(cx)))));
    let channel = scope
        .borrow()
        .as_ref()
        .unwrap()
        .channel(CommandPolicy::Concurrent);
    let owner = Rc::downgrade(&scope);
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    channel
        .submit(
            move |_| {
                let scope = owner.upgrade().unwrap().take();
                drop(scope);
                async move { drop(capture) }
            },
            |(), _| panic!("closed factory delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert!(channel.is_closed());
    assert!(weak.upgrade().is_none());
}

#[gpui::test]
fn cancellation_of_started_future_can_submit_from_capture_destruction(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let reenter = channel.clone();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    let capture = OnDrop(Some(Box::new(move || {
        reenter
            .submit(
                |_| async { 5 },
                move |value, _| {
                    seen.borrow_mut().push(value);
                    Ok(())
                },
                unexpected,
            )
            .unwrap();
    })));
    let gate = Gate::default();
    let ticket = channel
        .submit(
            move |_| async move {
                gate.wait().await;
                drop(capture);
            },
            |(), _| panic!("cancelled future delivered"),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert!(values.borrow().is_empty());
    assert!(ticket.cancel());
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[5]);
}

#[gpui::test]
fn scope_reset_during_replacement_cleanup_invalidates_reserved_request(cx: &mut TestAppContext) {
    let scope = Rc::new(cx.update(|cx| CommandScope::new(cx)));
    let channel = scope.channel(CommandPolicy::Latest);
    let owner = Rc::downgrade(&scope);
    let capture = OnDrop(Some(Box::new(move || {
        owner.upgrade().unwrap().invalidate();
    })));
    channel
        .submit(
            move |_| {
                drop(capture);
                async {}
            },
            |(), _| panic!("replaced factory delivered"),
            unexpected,
        )
        .unwrap();
    let reset = channel
        .submit(
            |_| async {},
            |(), _| panic!("reset request delivered"),
            unexpected,
        )
        .unwrap();
    assert!(!reset.is_pending());
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    let next = channel
        .submit(
            |_| async { 6 },
            move |value, _| {
                seen.borrow_mut().push(value);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    assert!(next.id() > reset.id());
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[6]);
}
