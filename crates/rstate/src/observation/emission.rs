use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
    rc::Rc,
};

use crate::{StoreResult, Subscription};

type Listener<M> = dyn Fn(Rc<M>) -> StoreResult<()>;
type Filter<M> = dyn Fn(&M) -> bool;

struct Entry<M> {
    filter: Option<Box<Filter<M>>>,
    listener: Box<Listener<M>>,
}

pub(crate) struct Emissions<M> {
    inner: Rc<EmissionEntries<M>>,
}

struct EmissionEntries<M> {
    next_id: Cell<u64>,
    entries: RefCell<BTreeMap<u64, Rc<Entry<M>>>>,
}

impl<M> Default for Emissions<M> {
    fn default() -> Self {
        Self {
            inner: Rc::new(EmissionEntries {
                next_id: Cell::new(0),
                entries: RefCell::new(BTreeMap::new()),
            }),
        }
    }
}

impl<M: 'static> Emissions<M> {
    fn register(&self, entry: Entry<M>) -> Subscription {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id.checked_add(1).unwrap());
        self.inner.entries.borrow_mut().insert(id, Rc::new(entry));
        let weak = Rc::downgrade(&self.inner);
        Subscription::new(move || {
            if let Some(inner) = weak.upgrade() {
                let removed = inner.entries.borrow_mut().remove(&id);
                drop(removed);
            }
        })
    }

    pub(crate) fn on(
        &self,
        filter: impl Fn(&M) -> bool + 'static,
        listener: impl Fn(Rc<M>) -> StoreResult<()> + 'static,
    ) -> Subscription {
        self.register(Entry {
            filter: Some(Box::new(filter)),
            listener: Box::new(listener),
        })
    }

    pub(crate) fn on_any(
        &self,
        listener: impl Fn(Rc<M>) -> StoreResult<()> + 'static,
    ) -> Subscription {
        self.register(Entry {
            filter: None,
            listener: Box::new(listener),
        })
    }

    pub(crate) fn emit(&self, message: Rc<M>) -> StoreResult<()> {
        self.emit_tier(message.clone(), false)?;
        self.emit_tier(message, true)
    }

    fn emit_tier(&self, message: Rc<M>, wildcard: bool) -> StoreResult<()> {
        let mut cursor = None;
        loop {
            let next = {
                let entries = self.inner.entries.borrow();
                let lower = cursor.map_or(Unbounded, Excluded);
                entries
                    .range((lower, Unbounded))
                    .find(|(_, entry)| entry.filter.is_none() == wildcard)
                    .map(|(id, entry)| (*id, entry.clone()))
            };
            let Some((id, entry)) = next else {
                return Ok(());
            };
            cursor = Some(id);
            if entry.filter.as_ref().is_none_or(|filter| filter(&message)) {
                (entry.listener)(message.clone())?;
            }
        }
    }
}
