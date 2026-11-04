use crate::StoreResult;

pub trait SnapshotStorage {
    fn read(&self, name: &str) -> StoreResult<Option<Vec<u8>>>;

    fn write(&self, name: &str, bytes: &[u8]) -> StoreResult<()>;

    fn remove(&self, name: &str) -> StoreResult<()>;
}
