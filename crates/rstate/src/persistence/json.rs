use std::{fmt, marker::PhantomData};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{StoreError, StoreResult};

use super::{Persisted, SnapshotCodec};

pub struct JsonCodec<C> {
    marker: PhantomData<fn() -> C>,
}

impl<C> JsonCodec<C> {
    pub const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<C> Default for JsonCodec<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> Clone for JsonCodec<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for JsonCodec<C> {}

impl<C> fmt::Debug for JsonCodec<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonCodec").finish()
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope<C> {
    context: C,
    version: u64,
}

impl<C: Serialize + DeserializeOwned> SnapshotCodec<C> for JsonCodec<C> {
    fn encode(&self, context: &C, version: u64) -> StoreResult<Vec<u8>> {
        serde_json::to_vec(&Envelope { context, version }).map_err(StoreError::from_error)
    }

    fn decode(&self, bytes: &[u8]) -> StoreResult<Persisted<C>> {
        let envelope: Envelope<C> =
            serde_json::from_slice(bytes).map_err(StoreError::from_error)?;
        Ok(Persisted {
            context: envelope.context,
            version: envelope.version,
        })
    }
}
