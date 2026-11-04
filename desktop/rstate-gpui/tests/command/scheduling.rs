use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rstate_gpui::{CommandPolicy, CommandScope};

use super::support::{Gate, unexpected};

#[gpui::test]
fn factories_start_deferred_fifo_and_scope_owns_dropped_tickets(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let trace = Rc::new(RefCell::new(Vec::new()));
    cx.update(|_| {
        for value in [1_i32, 2, 3] {
            let started = trace.clone();
            let delivered = trace.clone();
            let ticket = channel
                .submit(
                    move |_| {
                        started.borrow_mut().push(value);
                        async move { value }
                    },
                    move |value, _| {
                        delivered.borrow_mut().push(value.saturating_mul(10));
                        Ok(())
                    },
                    unexpected,
                )
                .unwrap();
            drop(ticket);
        }
        assert!(trace.borrow().is_empty());
    });
    cx.run_until_parked();
    assert_eq!(&trace.borrow()[..3], &[1, 2, 3]);
    let mut delivered = trace.borrow()[3..].to_vec();
    delivered.sort_unstable();
    assert_eq!(delivered, [10, 20, 30]);
}

#[gpui::test]
fn independent_channels_complete_out_of_order(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let first = scope.channel(CommandPolicy::Concurrent);
    let second = scope.channel(CommandPolicy::Concurrent);
    let gate = Gate::default();
    let wait = gate.clone();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    let first_ticket = first
        .submit(
            move |_| async move {
                wait.wait().await;
                1
            },
            move |value, _| {
                seen.borrow_mut().push(value);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    let seen = values.clone();
    let second_ticket = second
        .submit(
            |_| async { 2 },
            move |value, _| {
                seen.borrow_mut().push(value);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    assert!(second_ticket.id() > first_ticket.id());
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[2]);
    assert!(first_ticket.is_pending());
    assert!(!second_ticket.is_pending());
    gate.release();
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[2, 1]);
    assert!(!first_ticket.is_pending());
}

#[gpui::test]
fn latest_cancels_queued_and_running_predecessors(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Latest);
    let starts = Rc::new(RefCell::new(Vec::new()));
    let values = Rc::new(RefCell::new(Vec::new()));
    let mut tickets = Vec::new();
    let mut gates = Vec::new();
    for value in [1, 2] {
        let gate = Gate::default();
        let wait = gate.clone();
        let started = starts.clone();
        let delivered = values.clone();
        tickets.push(
            channel
                .submit(
                    move |_| {
                        started.borrow_mut().push(value);
                        async move {
                            wait.wait().await;
                            value
                        }
                    },
                    move |value, _| {
                        delivered.borrow_mut().push(value);
                        Ok(())
                    },
                    unexpected,
                )
                .unwrap(),
        );
        gates.push(gate);
    }
    cx.run_until_parked();
    assert_eq!(&*starts.borrow(), &[2]);
    assert!(!tickets[0].is_pending());
    assert!(tickets[1].is_pending());
    let delivered = values.clone();
    channel
        .submit(
            |_| async { 3 },
            move |value, _| {
                delivered.borrow_mut().push(value);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    for gate in gates {
        gate.release();
    }
    cx.run_until_parked();
    assert_eq!(&*values.borrow(), &[3]);
    assert!(!tickets[1].is_pending());
}

#[gpui::test]
fn serial_factories_wait_for_completion_and_delivery(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Serial);
    let gate = Gate::default();
    let wait = gate.clone();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let started = trace.clone();
    let delivered = trace.clone();
    let reenter = channel.clone();
    channel
        .submit(
            move |_| {
                started.borrow_mut().push(1);
                async move { wait.wait().await }
            },
            move |(), _| {
                delivered.borrow_mut().push(11);
                let started = delivered.clone();
                let delivered = delivered.clone();
                reenter
                    .submit(
                        move |_| {
                            started.borrow_mut().push(3);
                            async {}
                        },
                        move |(), _| {
                            delivered.borrow_mut().push(13);
                            Ok(())
                        },
                        unexpected,
                    )
                    .unwrap();
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    let started = trace.clone();
    let delivered = trace.clone();
    channel
        .submit(
            move |_| {
                started.borrow_mut().push(2);
                async {}
            },
            move |(), _| {
                delivered.borrow_mut().push(12);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(&*trace.borrow(), &[1]);
    gate.release();
    cx.run_until_parked();
    assert_eq!(&*trace.borrow(), &[1, 11, 2, 12, 3, 13]);
}

#[gpui::test]
fn cancelled_serial_work_drains_before_next_factory(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Serial);
    let gate = Gate::default();
    let wait = gate.clone();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let started = trace.clone();
    let ticket = channel
        .submit(
            move |_| {
                started.borrow_mut().push(1);
                async move { wait.wait().await }
            },
            |(), _| panic!("cancelled output delivered"),
            unexpected,
        )
        .unwrap();
    let started = trace.clone();
    let delivered = trace.clone();
    channel
        .submit(
            move |_| {
                started.borrow_mut().push(2);
                async {}
            },
            move |(), _| {
                delivered.borrow_mut().push(3);
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert!(ticket.cancel());
    assert!(!ticket.cancel());
    assert!(!ticket.is_pending());
    cx.run_until_parked();
    assert_eq!(&*trace.borrow(), &[1]);
    gate.release();
    cx.run_until_parked();
    assert_eq!(&*trace.borrow(), &[1, 2, 3]);
}
