use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, PersistenceError, PersistenceOperation, Store, WeakStore};

use super::support::{Codec, Context, Event, Storage, config, persistence};

#[test]
fn nested_subscriber_commits_win_over_stale_outer_writes() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let weak = store.downgrade();
    let _subscription = store.subscribe(move |snapshot| {
        if snapshot.context.value == 1 {
            weak.upgrade().unwrap().send(Event::Add(10))?;
        }
        Ok(())
    });
    store.send(Event::Add(1)).unwrap();
    assert_eq!(store.get_snapshot().context.value, 11);
    assert_eq!(storage.saved().as_deref(), Some("1:11"));
    assert_eq!(*storage.calls.borrow(), ["read:state", "write:1:11"]);
}

#[test]
fn inspector_and_codec_clears_invalidate_outer_save_without_mutating_live_state() {
    for from_codec in [false, true] {
        let storage = Storage::default();
        storage.seed("1:8");
        let codec = Codec::default();
        let store = Store::new_persisted(
            config(0, Rc::new(RefCell::new(Vec::new()))),
            persistence(&storage, &codec),
        )
        .unwrap();
        let weak = store.downgrade();
        if from_codec {
            codec.on_encode.replace(Some(Rc::new(move || {
                weak.upgrade().unwrap().clear_storage().unwrap();
            })));
        } else {
            let _inspection = store
                .inspect(move |record| {
                    if matches!(record, Inspection::Transition { .. }) {
                        weak.upgrade().unwrap().clear_storage()?;
                    }
                    Ok(())
                })
                .unwrap();
        }
        store.send(Event::Add(1)).unwrap();
        assert_eq!(store.get_snapshot().context.value, 9);
        assert_eq!(storage.saved(), None);
        assert_eq!(*storage.calls.borrow(), ["read:state", "clear:state"]);
    }
}

#[test]
fn codec_reentry_discards_old_encoding_and_storage_reentry_drains_newest_write() {
    for from_codec in [false, true] {
        let storage = Storage::default();
        let codec = Codec::default();
        let store = Store::new_persisted(
            config(0, Rc::new(RefCell::new(Vec::new()))),
            persistence(&storage, &codec),
        )
        .unwrap();
        let weak = store.downgrade();
        let hook: Rc<dyn Fn()> = Rc::new(move || {
            weak.upgrade().unwrap().send(Event::Add(10)).unwrap();
        });
        if from_codec {
            codec.on_encode.replace(Some(hook));
        } else {
            storage.on_write.replace(Some(hook));
        }
        store.send(Event::Add(1)).unwrap();
        assert_eq!(store.get_snapshot().context.value, 11);
        assert_eq!(storage.saved().as_deref(), Some("1:11"));
        let expected = if from_codec {
            vec!["read:state", "write:1:11"]
        } else {
            vec!["read:state", "write:1:1", "write:1:11"]
        };
        assert_eq!(*storage.calls.borrow(), expected);
    }
}

#[test]
fn explicit_save_reports_a_newer_queued_failure_instead_of_claiming_success() {
    for from_codec in [false, true] {
        let storage = Storage::default();
        let codec = Codec::default();
        let store = Store::new_persisted(
            config(0, Rc::new(RefCell::new(Vec::new()))),
            persistence(&storage, &codec),
        )
        .unwrap();
        let weak = store.downgrade();
        let backend = storage.clone();
        let hook: Rc<dyn Fn()> = Rc::new(move || {
            if from_codec {
                backend.fail_write.set(true);
            }
            weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
            if !from_codec {
                let backend = backend.clone();
                let failure = backend.fail_write.clone();
                backend.on_write.replace(Some(Rc::new(move || {
                    failure.set(true);
                })));
            }
        });
        if from_codec {
            codec.on_encode.replace(Some(hook));
        } else {
            storage.on_write.replace(Some(hook));
        }
        let error = store.save().unwrap_err();
        let source = std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<PersistenceError>()
            .unwrap();
        assert!(matches!(
            source,
            PersistenceError::Operation {
                operation: PersistenceOperation::Save,
                ..
            }
        ));
        assert_eq!(store.get_snapshot().context.value, 1);
        assert!(store.persistence_error().is_some());
        assert_eq!(
            storage.saved().as_deref(),
            if from_codec { None } else { Some("1:0") }
        );
    }
}

#[test]
fn explicit_save_returns_superseded_when_its_codec_clears_storage() {
    let storage = Storage::default();
    storage.seed("1:4");
    let codec = Codec::default();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &codec),
    )
    .unwrap();
    let weak = store.downgrade();
    codec.on_encode.replace(Some(Rc::new(move || {
        weak.upgrade().unwrap().clear_storage().unwrap();
    })));
    let error = store.save().unwrap_err();
    assert!(matches!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<PersistenceError>(),
        Some(PersistenceError::Superseded)
    ));
    assert_eq!(storage.saved(), None);
    assert_eq!(store.get_snapshot().context.value, 4);
    assert!(store.persistence_error().is_none());
    store.save().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:4"));
}

#[test]
fn explicit_save_succeeds_when_a_newer_reentrant_commit_is_actually_persisted() {
    let storage = Storage::default();
    let codec = Codec::default();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &codec),
    )
    .unwrap();
    let weak = store.downgrade();
    codec.on_encode.replace(Some(Rc::new(move || {
        weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
    })));
    store.save().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:1"));
    assert_eq!(store.get_snapshot().context.value, 1);
    assert!(store.persistence_error().is_none());
}

#[test]
fn explicit_operations_during_storage_callbacks_return_typed_errors_without_deadlock() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let recorded = errors.clone();
    let weak = store.downgrade();
    storage.on_write.replace(Some(Rc::new(move || {
        let store = weak.upgrade().unwrap();
        for result in [store.save(), store.clear_storage()] {
            let error = result.unwrap_err();
            let original = std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<PersistenceError>()
                .unwrap();
            if let PersistenceError::Reentrant { operation } = original {
                recorded.borrow_mut().push(*operation);
            }
        }
    })));
    store.send(Event::Add(1)).unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:1"));
    assert_eq!(
        *errors.borrow(),
        [PersistenceOperation::Save, PersistenceOperation::Clear]
    );
    assert!(store.persistence_error().is_none());
}

#[test]
fn error_callbacks_can_commit_a_recovery_without_cycles_or_stale_failure_status() {
    let storage = Storage::default();
    let handle: Rc<RefCell<Option<WeakStore<Context, Event>>>> = Rc::default();
    let current = handle.clone();
    let backend = storage.clone();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &Codec::default()).on_error(move |_| {
            backend.fail_write.set(false);
            let weak = current.borrow().as_ref().unwrap().clone();
            weak.upgrade().unwrap().send(Event::Add(1)).unwrap();
        }),
    )
    .unwrap();
    handle.replace(Some(store.downgrade()));
    storage.fail_write.set(true);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(store.get_snapshot().context.value, 2);
    assert_eq!(storage.saved().as_deref(), Some("1:2"));
    assert!(store.persistence_error().is_none());
    let weak = store.downgrade();
    drop(store);
    assert!(weak.upgrade().is_none());
}

#[test]
fn clear_errors_are_explicit_and_new_commits_during_clear_are_saved_after_removal() {
    let storage = Storage::default();
    let store = Store::new_persisted(
        config(0, Rc::new(RefCell::new(Vec::new()))),
        persistence(&storage, &Codec::default()),
    )
    .unwrap();
    store.save().unwrap();
    storage.fail_clear.set(true);
    assert!(store.clear_storage().is_err());
    assert_eq!(storage.saved().as_deref(), Some("1:0"));
    storage.fail_clear.set(false);
    let weak = store.downgrade();
    storage.on_clear.replace(Some(Rc::new(move || {
        weak.upgrade().unwrap().send(Event::Add(3)).unwrap();
    })));
    store.clear_storage().unwrap();
    assert_eq!(store.get_snapshot().context.value, 3);
    assert_eq!(storage.saved().as_deref(), Some("1:3"));
}
