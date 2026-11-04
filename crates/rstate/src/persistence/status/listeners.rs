use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use crate::Subscription;

use super::PersistenceStatus;

type Callback = dyn Fn(Rc<PersistenceStatus>);

struct Listener {
    active: Cell<bool>,
    running: Cell<bool>,
    sequence: Cell<u64>,
    callback: Box<Callback>,
}

#[derive(Default)]
struct Entries {
    next: Cell<u64>,
    listeners: RefCell<BTreeMap<u64, Rc<Listener>>>,
}

#[derive(Default)]
pub struct Listeners {
    entries: Rc<Entries>,
}

impl Listeners {
    pub(super) fn subscribe(
        &self,
        sequence: u64,
        callback: impl Fn(Rc<PersistenceStatus>) + 'static,
    ) -> Subscription {
        let id = self.entries.next.get();
        #[expect(
            clippy::expect_used,
            reason = "exhausting persistence listener IDs cannot preserve registration order"
        )]
        self.entries.next.set(
            id.checked_add(1)
                .expect("persistence listener IDs exhausted"),
        );
        self.entries.listeners.borrow_mut().insert(
            id,
            Rc::new(Listener {
                active: Cell::new(true),
                running: Cell::new(false),
                sequence: Cell::new(sequence),
                callback: Box::new(callback),
            }),
        );
        let weak = Rc::downgrade(&self.entries);
        Subscription::new(move || {
            if let Some(entries) = weak.upgrade() {
                let removed = entries.listeners.borrow_mut().remove(&id);
                if let Some(listener) = removed {
                    listener.active.set(false);
                }
            }
        })
    }

    pub(super) fn publish(&self, current: impl Fn() -> Rc<PersistenceStatus>) {
        let listeners: Vec<_> = self.entries.listeners.borrow().values().cloned().collect();
        for listener in listeners {
            if !listener.active.get() || listener.running.get() {
                continue;
            }
            let status = current();
            if listener.sequence.get() == status.sequence() {
                continue;
            }
            listener.sequence.set(status.sequence());
            listener.running.set(true);
            let running = Running(&listener.running);
            (listener.callback)(status);
            drop(running);
            listener.sequence.set(current().sequence());
        }
    }
}

struct Running<'a>(&'a Cell<bool>);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
