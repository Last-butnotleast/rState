use std::rc::Rc;

use crate::{PersistenceError, Snapshot, StoreError, StoreResult};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Persisted<C> {
    pub context: C,
    pub version: u64,
}

impl<C> Persisted<C> {
    pub fn into_snapshot(self, expected_version: u64) -> StoreResult<Rc<Snapshot<C>>> {
        if self.version != expected_version {
            return Err(StoreError::from_error(PersistenceError::VersionMismatch {
                expected: expected_version,
                actual: self.version,
            }));
        }
        Ok(Rc::new(Snapshot::new(Rc::new(self.context))))
    }
}

pub trait SnapshotCodec<C> {
    fn encode(&self, context: &C, version: u64) -> StoreResult<Vec<u8>>;

    fn decode(&self, bytes: &[u8]) -> StoreResult<Persisted<C>>;
}
