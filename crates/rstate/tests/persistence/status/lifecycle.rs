use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{Store, Subscription, WeakStore};

use super::super::support::{Codec, Context, Event, Storage, config, persistence};

struct OnDrop {
    store: WeakStore<Context, Event>,
    count: Rc<Cell<u32>>,
}

impl Drop for OnDrop {
    fn drop(&mut self) {
        self.count.set(self.count.get().saturating_add(1));
        if let Some(store) = self.store.upgrade() {
            if let Ok(subscription) = store.subscribe_persistence(|_| {}) {
                let _guard = subscription.scoped();
                let _ = store.save();
            }
        }
    }
}

#[test]
fn removing_feedback_releases_captures_outside_registry_borrows() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let count = Rc::new(Cell::new(0));
    let capture = OnDrop {
        store: store.downgrade(),
        count: count.clone(),
    };
    let guard = store
        .subscribe_persistence(move |_| {
            let _ = &capture;
        })
        .unwrap()
        .scoped();
    drop(guard);
    assert_eq!(count.get(), 1);
    assert_eq!(store.persistence_status().unwrap().sequence(), 1);
}

#[test]
fn live_removal_and_registration_preserve_no_replay_and_non_owning_lifetimes() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::default()),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let second: Rc<RefCell<Option<Subscription>>> = Rc::default();
    let other = second.clone();
    let weak = store.downgrade();
    let added = Rc::new(Cell::new(0_u32));
    let count = added.clone();
    let _first = store
        .subscribe_persistence(move |_| {
            if let Some(subscription) = other.borrow_mut().take() {
                subscription.unsubscribe();
            }
            let count = count.clone();
            weak.upgrade()
                .unwrap()
                .subscribe_persistence(move |_| count.set(count.get().saturating_add(1)))
                .unwrap();
        })
        .unwrap()
        .scoped();
    let removed = Rc::new(Cell::new(0_u32));
    let count = removed.clone();
    second.replace(Some(
        store
            .subscribe_persistence(move |_| count.set(count.get().saturating_add(1)))
            .unwrap(),
    ));
    store.save().unwrap();
    assert_eq!(removed.get(), 0);
    assert_eq!(added.get(), 0);
    store.save().unwrap();
    assert_eq!(added.get(), 1);
    let weak = store.downgrade();
    drop(store);
    assert!(weak.upgrade().is_none());
}

type ErrorDropHook = (u32, Box<dyn Fn()>);

thread_local! {
    static ERROR_DROP: RefCell<Option<ErrorDropHook>> = const { RefCell::new(None) };
}

#[derive(Debug)]
struct ReenterError(u32);

impl std::fmt::Display for ReenterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("drop reentry failure")
    }
}

impl std::error::Error for ReenterError {}

impl Drop for ReenterError {
    fn drop(&mut self) {
        let callback = ERROR_DROP.with_borrow_mut(|hook| {
            if hook.as_ref().is_some_and(|(id, _)| *id == self.0) {
                hook.take().map(|(_, callback)| callback)
            } else {
                None
            }
        });
        if let Some(callback) = callback {
            callback();
        }
    }
}

struct DropStorage(Storage, Cell<u32>);

impl rstate::SnapshotStorage for DropStorage {
    fn read(&self, name: &str) -> rstate::StoreResult<Option<Vec<u8>>> {
        rstate::SnapshotStorage::read(&self.0, name)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> rstate::StoreResult<()> {
        if self.0.fail_write.get() {
            let id = self.1.get().saturating_add(1);
            self.1.set(id);
            Err(rstate::StoreError::from_error(ReenterError(id)))
        } else {
            rstate::SnapshotStorage::write(&self.0, name, bytes)
        }
    }

    fn remove(&self, name: &str) -> rstate::StoreResult<()> {
        rstate::SnapshotStorage::remove(&self.0, name)
    }
}

#[test]
fn replacing_a_failed_status_releases_error_captures_after_the_final_busy_guard() {
    for clear in [false, true] {
        let storage = Storage::default();
        let store = Store::new_persisted(
            config(0, Rc::default()),
            rstate::Persistence::new(
                "state",
                1,
                DropStorage(storage.clone(), Cell::new(0)),
                Codec::default(),
            ),
        )
        .unwrap();
        storage.fail_write.set(true);
        assert!(store.save().is_err());
        storage.fail_write.set(false);
        let weak = store.downgrade();
        ERROR_DROP.with_borrow_mut(|callback| {
            *callback = Some((
                1,
                Box::new(move || {
                    weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
                }),
            ));
        });
        if clear {
            store.clear_storage().unwrap();
        } else {
            store.save().unwrap();
        }
        assert_eq!(store.get_snapshot().context.value, 1);
        assert_eq!(storage.saved().as_deref(), Some("1:1"));
        let status = store.persistence_status().unwrap();
        assert!(matches!(
            status.outcome(),
            Some(rstate::PersistenceOutcome::Succeeded)
        ));
        assert_eq!(
            status.last_activity(),
            Some(rstate::PersistenceActivity::AutomaticSave)
        );
    }
}

#[test]
fn discarded_drain_errors_cannot_enqueue_unsaved_events_after_the_final_pending_check() {
    for operation in ["automatic", "clear", "save"] {
        let storage = Storage::default();
        let slot: Rc<RefCell<Option<WeakStore<Context, Event>>>> = Rc::default();
        let current = slot.clone();
        let backend = storage.clone();
        let failures = Rc::new(Cell::new(0_u32));
        let errors = failures.clone();
        let store = Store::new_persisted(
            config(0, Rc::default()),
            rstate::Persistence::new(
                "state",
                1,
                DropStorage(storage.clone(), Cell::new(0)),
                Codec::default(),
            )
            .on_error(move |_| {
                let count = errors.get().saturating_add(1);
                errors.set(count);
                if count == 2 {
                    backend.fail_write.set(false);
                }
                let weak = current.borrow().as_ref().unwrap().clone();
                weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
            }),
        )
        .unwrap();
        slot.replace(Some(store.downgrade()));
        let weak = store.downgrade();
        let id = if operation == "clear" { 1 } else { 2 };
        ERROR_DROP.with_borrow_mut(|callback| {
            *callback = Some((
                id,
                Box::new(move || {
                    weak.upgrade().unwrap().send(Event::Add(10)).unwrap();
                }),
            ));
        });
        if operation == "clear" {
            let weak = store.downgrade();
            let backend = storage.clone();
            storage.on_clear.replace(Some(Rc::new(move || {
                backend.fail_write.set(true);
                weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
            })));
            store.clear_storage().unwrap();
        } else {
            storage.fail_write.set(true);
            if operation == "automatic" {
                store.send(Event::Add(1)).unwrap();
            } else {
                assert!(store.save().is_err());
            }
        }
        assert_eq!(failures.get(), 2);
        let value = store.get_snapshot().context.value;
        assert!(value >= 12);
        assert_eq!(storage.saved(), Some(format!("1:{value}")), "{operation}");
        assert!(matches!(
            store.persistence_status().unwrap().outcome(),
            Some(rstate::PersistenceOutcome::Succeeded)
        ));
    }
}
