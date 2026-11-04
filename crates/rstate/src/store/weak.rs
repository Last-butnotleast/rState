use std::{
    fmt,
    rc::{Rc, Weak},
};

use super::handle::{Store, StoreInner};

pub struct WeakStore<C, E, M = ()> {
    inner: Weak<StoreInner<C, E, M>>,
}

impl<C, E, M> Store<C, E, M> {
    pub fn downgrade(&self) -> WeakStore<C, E, M> {
        WeakStore {
            inner: Rc::downgrade(&self.inner),
        }
    }
}

impl<C, E, M> Clone for WeakStore<C, E, M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<C, E, M> fmt::Debug for WeakStore<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakStore")
            .field("alive", &(self.inner.strong_count() != 0))
            .finish_non_exhaustive()
    }
}

impl<C, E, M> WeakStore<C, E, M> {
    pub fn upgrade(&self) -> Option<Store<C, E, M>> {
        self.inner.upgrade().map(|inner| Store { inner })
    }
}
