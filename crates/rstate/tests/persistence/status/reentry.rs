use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{
    DispatchStatus, PersistenceActivity, PersistenceOutcome, Store, StoreError, WeakStore,
};

use super::super::support::{Codec, Context, Event, Storage, config, persistence};

#[test]
fn feedback_retry_runs_after_io_and_reconciles_later_observers_to_live_recovery() {
    let storage = Storage::default();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new_persisted(
        config(0, effects.clone()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let weak = store.downgrade();
    let backend = storage.clone();
    let seen = trace.clone();
    let _first = store
        .subscribe_persistence(move |status| {
            seen.borrow_mut().push(format!("a:{}", status.sequence()));
            if status.error().is_some() {
                backend.fail_write.set(false);
                weak.upgrade().unwrap().save().unwrap();
                seen.borrow_mut().push("a-return".into());
            }
        })
        .unwrap()
        .scoped();
    let seen = trace.clone();
    let _second = store
        .subscribe_persistence(move |status| {
            seen.borrow_mut().push(format!("b:{}", status.sequence()));
        })
        .unwrap()
        .scoped();
    storage.fail_write.set(true);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(&*trace.borrow(), &["a:1", "b:2", "a-return"]);
    let current = store.persistence_status().unwrap();
    assert_eq!(current.last_activity(), Some(PersistenceActivity::Save));
    assert!(matches!(
        current.outcome(),
        Some(PersistenceOutcome::Succeeded)
    ));
    assert!(current.error().is_none());
    assert_eq!(&*effects.borrow(), &["effect"]);
    assert_eq!(storage.saved().as_deref(), Some("1:1"));
}

#[test]
fn reentrant_rejections_are_readable_and_delivered_only_after_storage_returns() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let active = Rc::new(Cell::new(false));
    let in_io = active.clone();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let seen = observed.clone();
    let _guard = store
        .subscribe_persistence(move |status| {
            assert!(!in_io.get());
            seen.borrow_mut().push(status.sequence());
        })
        .unwrap()
        .scoped();
    let weak = store.downgrade();
    let in_io = active;
    storage.on_write.replace(Some(Rc::new(move || {
        in_io.set(true);
        let store = weak.upgrade().unwrap();
        assert!(store.save().is_err());
        let rejected = store.persistence_status().unwrap();
        assert_eq!(rejected.last_activity(), Some(PersistenceActivity::Save));
        assert!(matches!(
            rejected.outcome(),
            Some(PersistenceOutcome::Failed(_))
        ));
        assert!(rejected.error().is_none());
        assert!(store.clear_storage().is_err());
        assert_eq!(
            store.persistence_status().unwrap().last_activity(),
            Some(PersistenceActivity::Clear)
        );
        in_io.set(false);
    })));
    store.send(Event::Add(1)).unwrap();
    assert_eq!(&*observed.borrow(), &[3]);
    assert_eq!(
        store.persistence_status().unwrap().last_activity(),
        Some(PersistenceActivity::AutomaticSave)
    );
    assert!(store.persistence_error().is_none());
}

#[test]
fn legacy_error_callback_clear_or_recovery_wins_over_outer_automatic_status() {
    for clear in [false, true] {
        let storage = Storage::default();
        let slot: Rc<RefCell<Option<WeakStore<Context, Event>>>> = Rc::default();
        let target = slot.clone();
        let backend = storage.clone();
        let store = Store::new_persisted(
            config(0, Rc::default()),
            persistence(&storage, &Codec::default()).on_error(move |_| {
                backend.fail_write.set(false);
                let weak = target.borrow().as_ref().unwrap().clone();
                let store = weak.upgrade().unwrap();
                if clear {
                    store.clear_storage().unwrap();
                } else {
                    store.send(Event::Add(1)).unwrap();
                }
            }),
        )
        .unwrap();
        slot.replace(Some(store.downgrade()));
        storage.fail_write.set(true);
        store.send(Event::Add(1)).unwrap();
        let status = store.persistence_status().unwrap();
        assert_eq!(
            status.last_activity(),
            Some(if clear {
                PersistenceActivity::Clear
            } else {
                PersistenceActivity::AutomaticSave
            })
        );
        assert!(matches!(
            status.outcome(),
            Some(PersistenceOutcome::Succeeded)
        ));
        assert!(status.error().is_none());
        assert!(store.persistence_error().is_none());
    }
}

#[test]
fn persistence_feedback_does_not_drain_or_swallow_pending_domain_errors() {
    let storage = Storage::default();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new_persisted(
        config(0, effects.clone()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let seen = trace.clone();
    let _status = store
        .subscribe_persistence(move |_| seen.borrow_mut().push("status"))
        .unwrap()
        .scoped();
    let weak = store.downgrade();
    let seen = trace.clone();
    let _first = store
        .subscribe(move |_| {
            seen.borrow_mut().push("a-enter");
            weak.upgrade().unwrap().save()?;
            seen.borrow_mut().push("a-return");
            Ok(())
        })
        .scoped();
    let seen = trace.clone();
    let _second = store
        .subscribe(move |_| {
            seen.borrow_mut().push("b");
            Err(StoreError::new("domain delivery failed"))
        })
        .scoped();
    let error = store.send(Event::Add(1)).unwrap_err();
    assert_eq!(error.to_string(), "domain delivery failed");
    assert_eq!(error.dispatch_status(), Some(DispatchStatus::Committed));
    assert_eq!(&*trace.borrow(), &["a-enter", "status", "a-return", "b"]);
    assert!(effects.borrow().is_empty());
}

#[test]
fn returned_explicit_failure_remains_distinct_from_recovered_storage_error_state() {
    let storage = Storage::default();
    let slot: Rc<RefCell<Option<WeakStore<Context, Event>>>> = Rc::default();
    let current = slot.clone();
    let backend = storage.clone();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()).on_error(move |_| {
            backend.fail_write.set(false);
            let weak = current.borrow().as_ref().unwrap().clone();
            weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
        }),
    )
    .unwrap();
    slot.replace(Some(store.downgrade()));
    storage.fail_write.set(true);
    assert!(store.save().is_err());
    let status = store.persistence_status().unwrap();
    assert_eq!(status.last_activity(), Some(PersistenceActivity::Save));
    assert!(matches!(
        status.outcome(),
        Some(PersistenceOutcome::Failed(_))
    ));
    assert!(status.error().is_none());
    assert_eq!(storage.saved().as_deref(), Some("1:1"));
}

#[test]
fn in_flight_explicit_error_callback_reads_the_previous_status_and_live_error_channel() {
    let storage = Storage::default();
    let slot: Rc<RefCell<Option<WeakStore<Context, Event>>>> = Rc::default();
    let current = slot.clone();
    let observations = Rc::new(RefCell::new(Vec::new()));
    let seen = observations.clone();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()).on_error(move |_| {
            let weak = current.borrow().as_ref().unwrap().clone();
            let store = weak.upgrade().unwrap();
            let status = store.persistence_status().unwrap();
            seen.borrow_mut().push((
                status.sequence(),
                status.error().is_some(),
                store.persistence_error().is_some(),
            ));
        }),
    )
    .unwrap();
    slot.replace(Some(store.downgrade()));
    storage.fail_write.set(true);
    assert!(store.save().is_err());
    assert_eq!(&*observations.borrow(), &[(0, false, true)]);
    let status = store.persistence_status().unwrap();
    assert_eq!(status.sequence(), 1);
    assert_eq!(status.last_activity(), Some(PersistenceActivity::Save));
    assert!(Rc::ptr_eq(
        status.error().unwrap(),
        &store.persistence_error().unwrap()
    ));
}
