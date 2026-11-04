use std::sync::Arc;

use crate::StoreError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PersistenceOperation {
    Load,
    Encode,
    Decode,
    Save,
    Clear,
}

#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PersistenceError {
    #[error("persistence {operation:?} failed: {source}")]
    #[non_exhaustive]
    Operation {
        operation: PersistenceOperation,
        #[source]
        source: Arc<StoreError>,
    },
    #[error("persisted version {actual} does not match configured version {expected}")]
    #[non_exhaustive]
    VersionMismatch { expected: u64, actual: u64 },
    #[error("persistence {operation:?} cannot run during an active storage operation")]
    #[non_exhaustive]
    Reentrant { operation: PersistenceOperation },
    #[error("the store has no persistence configuration")]
    NotConfigured,
    #[error("persistence clear epochs are exhausted")]
    EpochExhausted,
    #[error("the save was superseded before the current state was persisted")]
    Superseded,
}

impl PersistenceError {
    pub(super) fn operation(operation: PersistenceOperation, source: StoreError) -> Self {
        Self::Operation {
            operation,
            source: Arc::new(source),
        }
    }
}
