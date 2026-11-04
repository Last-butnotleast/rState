use std::{fmt, rc::Rc};

use super::{PersistenceError, SnapshotCodec, SnapshotStorage};

pub(super) type ErrorListener = dyn Fn(&PersistenceError);

pub struct Persistence<C> {
    pub(super) name: String,
    pub(super) version: u64,
    pub(super) storage: Rc<dyn SnapshotStorage>,
    pub(super) codec: Rc<dyn SnapshotCodec<C>>,
    pub(super) on_error: Option<Rc<ErrorListener>>,
}

impl<C> Clone for Persistence<C> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            version: self.version,
            storage: self.storage.clone(),
            codec: self.codec.clone(),
            on_error: self.on_error.clone(),
        }
    }
}

impl<C> fmt::Debug for Persistence<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Persistence")
            .field("name", &self.name)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

impl<C> Persistence<C> {
    pub fn new(
        name: impl Into<String>,
        version: u64,
        storage: impl SnapshotStorage + 'static,
        codec: impl SnapshotCodec<C> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            version,
            storage: Rc::new(storage),
            codec: Rc::new(codec),
            on_error: None,
        }
    }

    #[must_use]
    pub fn on_error(mut self, listener: impl Fn(&PersistenceError) + 'static) -> Self {
        self.on_error = Some(Rc::new(listener));
        self
    }
}
