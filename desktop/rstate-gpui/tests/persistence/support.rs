use std::{cell::Cell, rc::Rc};

use rstate::{
    Persisted, Persistence, SnapshotCodec, SnapshotStorage, Store, StoreConfig, StoreError,
    StoreResult, Update,
};

use crate::support::Event;

#[derive(Clone, Default)]
pub struct Storage {
    pub fail: Rc<Cell<bool>>,
    pub writes: Rc<Cell<u32>>,
}

impl SnapshotStorage for Storage {
    fn read(&self, _: &str) -> StoreResult<Option<Vec<u8>>> {
        Ok(None)
    }

    fn write(&self, _: &str, _: &[u8]) -> StoreResult<()> {
        self.writes.set(self.writes.get().saturating_add(1));
        if self.fail.get() {
            Err(StoreError::new("storage unavailable"))
        } else {
            Ok(())
        }
    }

    fn remove(&self, _: &str) -> StoreResult<()> {
        Ok(())
    }
}

struct Codec;

impl SnapshotCodec<i32> for Codec {
    fn encode(&self, value: &i32, _: u64) -> StoreResult<Vec<u8>> {
        Ok(value.to_string().into_bytes())
    }

    fn decode(&self, _: &[u8]) -> StoreResult<Persisted<i32>> {
        Err(StoreError::new("no stored data"))
    }
}

pub fn store(storage: &Storage) -> StoreResult<Store<i32, Event>> {
    Store::new_persisted(
        StoreConfig::new(0, |_, event, _| match event {
            Event::Set(value) => Ok(Update::replace(*value)),
            Event::Keep => Ok(Update::Keep),
            Event::Fail => Err(StoreError::new("reducer failed")),
        }),
        Persistence::new("state", 1, storage.clone(), Codec),
    )
}
