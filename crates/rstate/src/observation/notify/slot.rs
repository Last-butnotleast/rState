use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use crate::Subscription;

use super::{Node, ObserverEntries, Observers};

#[derive(Clone)]
pub struct ObserverSlot {
    inner: Rc<Position>,
}

struct Position {
    source: Weak<ObserverEntries>,
    id: Cell<Option<u64>>,
}

impl Position {
    #[expect(
        clippy::expect_used,
        reason = "live dependency reservations cannot exceed the monotonic subscription ID space"
    )]
    fn reserve(&self) {
        if self.id.get().is_none() {
            if let Some(source) = self.source.upgrade() {
                let id = source.reserve();
                source.dependencies.set(
                    source
                        .dependencies
                        .get()
                        .checked_add(1)
                        .expect("observer dependency reservations exhausted"),
                );
                self.id.set(Some(id));
            }
        }
    }

    #[expect(
        clippy::expect_used,
        reason = "only an existing dependency reservation is released"
    )]
    fn clear(&self) {
        if self.id.take().is_some() {
            if let Some(source) = self.source.upgrade() {
                source.dependencies.set(
                    source
                        .dependencies
                        .get()
                        .checked_sub(1)
                        .expect("observer dependency reservation is active"),
                );
            }
        }
    }
}

impl Drop for Position {
    fn drop(&mut self) {
        self.clear();
    }
}

impl ObserverSlot {
    pub(super) fn new(source: Weak<ObserverEntries>) -> Self {
        Self {
            inner: Rc::new(Position {
                source,
                id: Cell::new(None),
            }),
        }
    }

    pub(crate) fn reserve(&self) {
        self.inner.reserve();
    }

    pub(crate) fn clear(&self) {
        self.inner.clear();
    }

    #[expect(
        clippy::expect_used,
        reason = "Selection retains its source and reserves its position before attachment"
    )]
    pub(crate) fn attach(&self, group: Observers) -> Subscription {
        self.reserve();
        let source = self
            .inner
            .source
            .upgrade()
            .expect("selection source is alive");
        let id = self.inner.id.get().expect("selection position is reserved");
        source.entries.borrow_mut().insert(id, Node::Group(group));
        let weak = self.inner.source.clone();
        Subscription::new(move || {
            if let Some(source) = weak.upgrade() {
                let node = source.entries.borrow_mut().remove(&id);
                drop(node);
            }
        })
    }
}
