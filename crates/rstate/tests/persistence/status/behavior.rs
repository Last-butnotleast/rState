use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{PersistenceActivity, PersistenceError, PersistenceOutcome, Store};

use super::super::support::{Codec, Event, Storage, config, persistence};

#[test]
fn initial_status_is_stable_without_replay_and_unconfigured_stores_report_absence() {
    let plain = Store::new(config(0, Rc::default()));
    assert!(plain.persistence_status().is_none());
    assert!(plain.subscribe_persistence(|_| {}).is_err());
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let initial = store.persistence_status().unwrap();
    assert_eq!(initial.sequence(), 0);
    assert!(initial.last_activity().is_none());
    assert!(initial.outcome().is_none());
    assert!(initial.error().is_none());
    assert!(Rc::ptr_eq(&initial, &store.persistence_status().unwrap()));
    let notifications = Rc::new(Cell::new(0_u32));
    let count = notifications.clone();
    let guard = store
        .subscribe_persistence(move |_| count.set(count.get().saturating_add(1)))
        .unwrap()
        .scoped();
    assert_eq!(notifications.get(), 0);
    store.save().unwrap();
    assert_eq!(notifications.get(), 1);
    drop(guard);
    store.save().unwrap();
    assert_eq!(notifications.get(), 1);
}

#[test]
fn automatic_failure_replacement_and_recovery_have_current_error_identity_and_keep_effects() {
    let storage = Storage::default();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let store = Store::new_persisted(
        config(0, effects.clone()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let seen = statuses.clone();
    let _guard = store
        .subscribe_persistence(move |status| seen.borrow_mut().push(status))
        .unwrap()
        .scoped();
    storage.fail_write.set(true);
    store.send(Event::Add(1)).unwrap();
    let first = store.persistence_status().unwrap();
    assert_eq!(
        first.last_activity(),
        Some(PersistenceActivity::AutomaticSave)
    );
    assert!(matches!(
        first.outcome(),
        Some(PersistenceOutcome::Failed(_))
    ));
    assert!(Rc::ptr_eq(
        first.error().unwrap(),
        &store.persistence_error().unwrap()
    ));
    store.send(Event::Add(1)).unwrap();
    let second = store.persistence_status().unwrap();
    assert!(!Rc::ptr_eq(first.error().unwrap(), second.error().unwrap()));
    storage.fail_write.set(false);
    store.send(Event::Add(1)).unwrap();
    let recovered = store.persistence_status().unwrap();
    assert_eq!(recovered.sequence(), 3);
    assert!(matches!(
        recovered.outcome(),
        Some(PersistenceOutcome::Succeeded)
    ));
    assert!(recovered.error().is_none());
    assert!(store.persistence_error().is_none());
    assert_eq!(statuses.borrow().len(), 3);
    assert_eq!(&*effects.borrow(), &["effect", "effect", "effect"]);
    assert_eq!(storage.saved().as_deref(), Some("1:3"));
}

#[test]
fn explicit_save_and_clear_report_outcomes_without_domain_snapshot_publication() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(4, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let snapshot = store.get_snapshot();
    let domain = Rc::new(Cell::new(0_u32));
    let count = domain.clone();
    let _guard = store
        .subscribe(move |_| {
            count.set(count.get().saturating_add(1));
            Ok(())
        })
        .scoped();
    store.save().unwrap();
    assert_eq!(
        store.persistence_status().unwrap().last_activity(),
        Some(PersistenceActivity::Save)
    );
    storage.fail_clear.set(true);
    assert!(store.clear_storage().is_err());
    let failed = store.persistence_status().unwrap();
    assert_eq!(failed.last_activity(), Some(PersistenceActivity::Clear));
    assert!(matches!(
        failed.outcome(),
        Some(PersistenceOutcome::Failed(_))
    ));
    storage.fail_clear.set(false);
    store.clear_storage().unwrap();
    let cleared = store.persistence_status().unwrap();
    assert_eq!(cleared.last_activity(), Some(PersistenceActivity::Clear));
    assert!(matches!(
        cleared.outcome(),
        Some(PersistenceOutcome::Succeeded)
    ));
    assert!(cleared.error().is_none());
    assert!(Rc::ptr_eq(&snapshot, &store.get_snapshot()));
    assert_eq!(domain.get(), 0);
    assert_eq!(storage.saved(), None);
}

#[test]
fn explicit_save_superseded_outcome_does_not_invent_a_storage_error() {
    let storage = Storage::default();
    let codec = Codec::default();
    let store =
        Store::new_persisted(config(4, Rc::default()), persistence(&storage, &codec)).unwrap();
    let weak = store.downgrade();
    codec.on_encode.replace(Some(Rc::new(move || {
        weak.upgrade().unwrap().clear_storage().unwrap();
    })));
    assert!(store.save().is_err());
    let status = store.persistence_status().unwrap();
    assert_eq!(status.last_activity(), Some(PersistenceActivity::Save));
    assert!(matches!(
        status.outcome(),
        Some(PersistenceOutcome::Superseded)
    ));
    assert!(status.error().is_none());
    assert!(store.persistence_error().is_none());
    assert_eq!(status.sequence(), 2);
    assert_eq!(storage.saved(), None);
}

#[test]
fn skipped_autosave_after_domain_delivery_failure_preserves_last_observation() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    store.save().unwrap();
    let saved = store.persistence_status().unwrap();
    let _guard = store
        .subscribe(|_| Err(rstate::StoreError::new("delivery failed")))
        .scoped();
    assert!(store.send(Event::Add(1)).is_err());
    assert!(Rc::ptr_eq(&saved, &store.persistence_status().unwrap()));
    assert_eq!(storage.saved().as_deref(), Some("1:0"));
    assert_eq!(store.get_snapshot().context.value, 1);
}

#[test]
fn explicit_save_reports_queued_failure_as_its_final_outcome() {
    let storage = Storage::default();
    let codec = Codec::default();
    let store =
        Store::new_persisted(config(0, Rc::default()), persistence(&storage, &codec)).unwrap();
    let weak = store.downgrade();
    let backend = storage;
    codec.on_encode.replace(Some(Rc::new(move || {
        backend.fail_write.set(true);
        weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
    })));
    assert!(store.save().is_err());
    let status = store.persistence_status().unwrap();
    assert_eq!(status.last_activity(), Some(PersistenceActivity::Save));
    assert!(
        matches!(status.outcome(), Some(PersistenceOutcome::Failed(error)) if matches!(error.as_ref(), PersistenceError::Operation { .. }))
    );
    assert!(status.error().is_some());
}

#[test]
fn successful_clear_can_report_a_nested_automatic_storage_failure_separately() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    store.save().unwrap();
    let weak = store.downgrade();
    let backend = storage.clone();
    storage.on_clear.replace(Some(Rc::new(move || {
        backend.fail_write.set(true);
        weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
    })));
    store.clear_storage().unwrap();
    let status = store.persistence_status().unwrap();
    assert_eq!(status.last_activity(), Some(PersistenceActivity::Clear));
    assert!(matches!(
        status.outcome(),
        Some(PersistenceOutcome::Succeeded)
    ));
    assert!(status.error().is_some());
    assert!(Rc::ptr_eq(
        status.error().unwrap(),
        &store.persistence_error().unwrap()
    ));
    assert_eq!(storage.saved(), None);
}
