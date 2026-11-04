use std::{fmt, rc::Rc};

use crate::{SchemaCheck, SchemaMetadata, StoreError, StoreResult};

type Check<T> = dyn Fn(&T) -> StoreResult<SchemaCheck>;

pub struct Schema<T> {
    metadata: Rc<SchemaMetadata>,
    check: Rc<Check<T>>,
}

impl<T> Clone for Schema<T> {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            check: self.check.clone(),
        }
    }
}

impl<T> fmt::Debug for Schema<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schema")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

impl<T> Schema<T> {
    pub fn new(
        metadata: SchemaMetadata,
        check: impl Fn(&T) -> StoreResult<SchemaCheck> + 'static,
    ) -> Self {
        Self {
            metadata: Rc::new(metadata),
            check: Rc::new(check),
        }
    }

    pub fn metadata(&self) -> &SchemaMetadata {
        &self.metadata
    }

    pub fn check(&self, value: &T) -> StoreResult<SchemaCheck> {
        (self.check)(value).map_err(StoreError::without_dispatch_status)
    }
}
