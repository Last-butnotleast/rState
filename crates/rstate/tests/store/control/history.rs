use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, Store, StoreFactory, Update};

use super::support::{Event, Trace, config, describe};

#[test]
fn history_records_noops_observably_while_dry_runs_preserve_live_state() {
    let trace: Trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(0, &trace).with_history(3));
    let initial = store.get_snapshot();
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let observed = statuses.clone();
    let _subscription = store.subscribe(move |snapshot| {
        observed.borrow_mut().push(snapshot.history_status());
        Ok(())
    });

    assert!(!store.can(Event::Keep).unwrap());
    let pure = store.transition(initial.clone(), Event::Keep).unwrap();
    assert_eq!(pure.snapshot.history_status().past, 1);
    assert!(!pure.allowed);
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);

    store.send(Event::Keep).unwrap();
    let first = store.get_snapshot();
    store.send(Event::Same).unwrap();
    assert!(Rc::ptr_eq(&initial.context, &first.context));
    assert!(!Rc::ptr_eq(&initial, &first));
    assert_eq!(store.history_status().past, 2);
    assert_eq!(statuses.borrow().len(), 2);
    assert_eq!(statuses.borrow()[0].past, 1);
    assert_eq!(statuses.borrow()[1].past, 2);
    assert_eq!(store.get_initial_snapshot().history_status().past, 0);
}

#[test]
fn bounded_history_supports_redo_and_new_noops_invalidate_the_branch() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(0, &trace).with_history(2));
    store.send(Event::Add(1)).unwrap();
    store.send(Event::Add(1)).unwrap();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(store.history_status().past, 2);
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 1);
    assert!(!store.can_undo());
    assert!(store.can_redo());
    let unchanged = store.get_snapshot();
    store.undo().unwrap();
    assert!(Rc::ptr_eq(&unchanged, &store.get_snapshot()));
    store.redo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
    assert_eq!(store.history_status().past, 1);
    store.send(Event::Keep).unwrap();
    assert!(!store.can_redo());
    assert_eq!(store.history_status().past, 2);
}

#[test]
fn macrosteps_record_once_and_reset_undo_redo_do_not_replay_effects() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(0, &trace).with_history(10));
    let emitted = trace.clone();
    let _emissions = store.on_any(move |message| {
        emitted.borrow_mut().push((*message).into());
        Ok(())
    });
    store.send(Event::Start).unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
    assert_eq!(store.history_status().past, 1);
    assert_eq!(*trace.borrow(), ["effect:2", "message"]);
    trace.borrow_mut().clear();

    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 0);
    store.redo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
    store.reset().unwrap();
    assert_eq!(store.get_snapshot().context.count, 0);
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
    assert!(trace.borrow().is_empty());
}

#[test]
fn failures_before_commit_do_not_record_and_after_commit_keep_history() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(0, &trace).with_history(3));
    assert!(store.send(Event::Fail).is_err());
    assert_eq!(store.history_status().past, 0);
    let _bad = store.subscribe(|_| Err(rstate::StoreError::new("subscriber failed")));
    let inspected = trace.clone();
    let _inspector = store
        .inspect(move |record| {
            inspected.borrow_mut().push(describe(record));
            Ok(())
        })
        .unwrap();
    trace.borrow_mut().clear();
    assert!(store.send(Event::Start).is_err());
    assert_eq!(store.get_snapshot().context.count, 2);
    assert_eq!(store.history_status().past, 1);
    assert!(trace.borrow().is_empty());
}

#[test]
fn nested_commits_record_their_actual_pre_state_and_inspect_captured_history() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new(config(0, &trace).with_history(10));
    let weak = store.downgrade();
    let subscription = store.subscribe(move |snapshot| {
        if snapshot.context.count == 1 {
            weak.upgrade().unwrap().send(Event::Add(10))?;
        }
        Ok(())
    });
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let observed = statuses.clone();
    let _inspector = store
        .inspect(move |record| {
            if let Inspection::Transition { snapshot, .. } = record {
                observed
                    .borrow_mut()
                    .push((snapshot.context.count, snapshot.history_status().past));
            }
            Ok(())
        })
        .unwrap();

    store.send(Event::Add(1)).unwrap();
    assert_eq!(*statuses.borrow(), [(11, 2), (1, 1)]);
    subscription.unsubscribe();
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 1);
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.count, 0);
}

#[test]
fn zero_capacity_and_disabled_history_have_explicit_empty_control_behavior() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let disabled = Store::new(config(0, &trace));
    let initial = disabled.get_snapshot();
    disabled.undo().unwrap();
    disabled.redo().unwrap();
    assert!(Rc::ptr_eq(&initial, &disabled.get_snapshot()));
    assert_eq!(disabled.history_status().limit, None);

    let store = Store::new(config(0, &trace).with_history(0));
    store.send(Event::Add(1)).unwrap();
    let current = store.get_snapshot();
    store.undo().unwrap();
    assert!(Rc::ptr_eq(&current, &store.get_snapshot()));
    assert_eq!(store.history_status().limit, Some(0));
    assert!(!store.can_undo());
    assert!(!store.can_redo());
}

#[test]
fn factory_history_is_instance_local_and_contexts_need_not_be_clone() {
    let factory: StoreFactory<i32, i32, i32> =
        StoreFactory::new(Ok, |context: &Rc<i32>, by: &i32, _| {
            Ok(Update::replace(context.saturating_add(*by)))
        })
        .with_history(2);
    let left = factory.create(1).unwrap();
    let right = factory.create(7).unwrap();
    left.send(2).unwrap();
    left.undo().unwrap();
    assert_eq!(*left.get_snapshot().context, 1);
    assert_eq!(*right.get_snapshot().context, 7);
    assert!(left.can_redo());
    assert!(!right.can_redo());
}
