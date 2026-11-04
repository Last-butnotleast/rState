use std::cell::Cell;

#[derive(Debug)]
pub struct Registration {
    active: Cell<bool>,
    dirty: Cell<bool>,
}

impl Registration {
    pub(crate) const fn new() -> Self {
        Self {
            active: Cell::new(true),
            dirty: Cell::new(false),
        }
    }

    pub(crate) const fn is_active(&self) -> bool {
        self.active.get()
    }

    pub(crate) const fn mark_dirty(&self) -> bool {
        !self.dirty.replace(true)
    }

    pub(crate) fn clear_dirty(&self) {
        self.dirty.set(false);
    }

    pub(crate) fn invalidate(&self) {
        self.active.set(false);
    }
}
