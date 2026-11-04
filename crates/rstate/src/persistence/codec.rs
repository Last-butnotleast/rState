use crate::StoreResult;

#[derive(Debug)]
pub struct Persisted<C> {
    pub context: C,
    pub version: u64,
}

pub trait SnapshotCodec<C> {
    fn encode(&self, context: &C, version: u64) -> StoreResult<Vec<u8>>;

    fn decode(&self, bytes: &[u8]) -> StoreResult<Persisted<C>>;
}
