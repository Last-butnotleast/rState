use std::{cell::Cell, rc::Rc};

use crate::{Snapshot, Store, StoreResult};

pub struct EffectScope<C, E, M = ()> {
    store: Store<C, E, M>,
    snapshot: Rc<Snapshot<C>>,
    committed: Rc<Cell<bool>>,
}

impl<C, E, M> Clone for EffectScope<C, E, M> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            snapshot: self.snapshot.clone(),
            committed: self.committed.clone(),
        }
    }
}

impl<C: 'static, E: 'static, M: 'static> EffectScope<C, E, M> {
    pub(crate) fn new(store: Store<C, E, M>, snapshot: Rc<Snapshot<C>>) -> Self {
        Self {
            store,
            snapshot,
            committed: Rc::new(Cell::new(false)),
        }
    }

    pub fn send(&self, event: E) -> StoreResult<()> {
        self.store.send(event)
    }

    pub fn get_snapshot(&self) -> Rc<Snapshot<C>> {
        if self.committed.get() {
            self.store.get_snapshot()
        } else {
            self.snapshot.clone()
        }
    }

    pub(crate) fn complete(&self) {
        self.committed.set(true);
    }
}
