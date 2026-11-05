use std::{cell::Cell, error::Error, rc::Rc};

use rstate::{
    Persisted, Persistence, PersistenceError, SnapshotCodec, Store, StoreConfig, StoreError,
    StoreResult, Update,
};

use super::support::Storage;

#[cfg(feature = "json")]
mod json;

#[test]
fn checked_restoration_accepts_non_clone_local_contexts_without_a_store() {
    struct Context {
        local: Rc<Cell<u64>>,
    }

    let local = Rc::new(Cell::new(7));
    let snapshot = Persisted {
        context: Context {
            local: local.clone(),
        },
        version: 3,
    }
    .into_snapshot(3)
    .unwrap();

    assert!(Rc::ptr_eq(&snapshot.context.local, &local));
    assert_eq!(snapshot.context.local.get(), 7);
    assert_eq!(snapshot.history_status().limit, None);
}

#[test]
fn unsupported_versions_return_the_existing_typed_error_without_dispatch_status() {
    let error = Persisted {
        context: 12,
        version: 9,
    }
    .into_snapshot(2)
    .unwrap_err();

    assert_eq!(error.dispatch_status(), None);
    assert!(matches!(
        error.source().and_then(|source| source.downcast_ref()),
        Some(PersistenceError::VersionMismatch {
            expected: 2,
            actual: 9,
            ..
        })
    ));
}

struct Legacy {
    count: i64,
    label: String,
}

#[derive(Debug)]
struct Current {
    count: u64,
    label: String,
}

fn migrate(persisted: Persisted<Legacy>) -> StoreResult<Persisted<Current>> {
    if persisted.version != 1 {
        return Err(StoreError::new(format!(
            "unsupported legacy version {}",
            persisted.version
        )));
    }
    Ok(Persisted {
        context: Current {
            count: u64::try_from(persisted.context.count).map_err(StoreError::from_error)?,
            label: persisted.context.label,
        },
        version: 2,
    })
}

#[test]
fn typed_application_migration_can_change_context_types_and_reject_invalid_data() {
    let snapshot = migrate(Persisted {
        context: Legacy {
            count: 15,
            label: "retained".into(),
        },
        version: 1,
    })
    .unwrap()
    .into_snapshot(2)
    .unwrap();
    assert_eq!(snapshot.context.count, 15);
    assert_eq!(snapshot.context.label, "retained");

    let unknown = migrate(Persisted {
        context: Legacy {
            count: 15,
            label: "unknown".into(),
        },
        version: 7,
    })
    .unwrap_err();
    assert_eq!(unknown.to_string(), "unsupported legacy version 7");

    let invalid = migrate(Persisted {
        context: Legacy {
            count: -1,
            label: "invalid".into(),
        },
        version: 1,
    })
    .unwrap_err();
    assert!(
        invalid
            .source()
            .unwrap()
            .downcast_ref::<std::num::TryFromIntError>()
            .is_some()
    );
    assert_eq!(invalid.dispatch_status(), None);
}

struct MigratingCodec {
    output_version: u64,
}

impl SnapshotCodec<Current> for MigratingCodec {
    fn encode(&self, context: &Current, version: u64) -> StoreResult<Vec<u8>> {
        Ok(format!("{version}:{}", context.count).into_bytes())
    }

    fn decode(&self, bytes: &[u8]) -> StoreResult<Persisted<Current>> {
        let text = std::str::from_utf8(bytes).map_err(StoreError::from_error)?;
        let (version, count) = text
            .split_once(':')
            .ok_or_else(|| StoreError::new("invalid legacy envelope"))?;
        let mut migrated = migrate(Persisted {
            context: Legacy {
                count: count.parse().map_err(StoreError::from_error)?,
                label: "stored".into(),
            },
            version: version.parse().map_err(StoreError::from_error)?,
        })?;
        migrated.version = self.output_version;
        Ok(migrated)
    }
}

#[test]
fn custom_codec_migration_still_obeys_the_store_loader_version_contract() {
    for version in [2, 3] {
        let storage = Storage::default();
        storage.seed("1:12");
        let config = StoreConfig::<_, ()>::new(
            Current {
                count: 0,
                label: "defaults".into(),
            },
            |_, &(), _| Ok(Update::Keep),
        );
        let persistence = Persistence::new(
            "state",
            2,
            storage.clone(),
            MigratingCodec {
                output_version: version,
            },
        );
        let restored = Store::new_persisted(config, persistence);
        if version == 2 {
            assert_eq!(restored.unwrap().get_snapshot().context.count, 12);
        } else {
            let error = restored.unwrap_err();
            assert!(matches!(
                error.source().and_then(|source| source.downcast_ref()),
                Some(PersistenceError::VersionMismatch {
                    expected: 2,
                    actual: 3,
                    ..
                })
            ));
        }
        assert_eq!(storage.saved().as_deref(), Some("1:12"));
        assert_eq!(*storage.calls.borrow(), ["read:state"]);
    }
}
