use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
    rc::Rc,
};

use crate::{ControlAction, Snapshot, StoreResult, Subscription};

use super::Inspection;

type Listener<C, E> = dyn for<'a> Fn(Inspection<'a, C, E>) -> StoreResult<()>;

pub struct Inspectors<C, E> {
    inner: Rc<InspectionEntries<C, E>>,
}

struct InspectionEntries<C, E> {
    next_id: Cell<u64>,
    entries: RefCell<BTreeMap<u64, Rc<Listener<C, E>>>>,
}

impl<C, E> Default for Inspectors<C, E> {
    fn default() -> Self {
        Self {
            inner: Rc::new(InspectionEntries {
                next_id: Cell::new(0),
                entries: RefCell::new(BTreeMap::new()),
            }),
        }
    }
}

impl<C: 'static, E: 'static> Inspectors<C, E> {
    pub(crate) fn inspect(
        &self,
        snapshot: Rc<Snapshot<C>>,
        listener: impl for<'a> Fn(Inspection<'a, C, E>) -> StoreResult<()> + 'static,
    ) -> StoreResult<Subscription> {
        let listener: Rc<Listener<C, E>> = Rc::new(listener);
        let subscription = self.register(listener.clone());
        listener(Inspection::Initial { snapshot })?;
        Ok(subscription)
    }

    fn register(&self, listener: Rc<Listener<C, E>>) -> Subscription {
        let id = self.inner.next_id.get();
        #[expect(
            clippy::expect_used,
            reason = "exhausting monotonic inspection IDs cannot preserve registration order"
        )]
        self.inner.next_id.set(
            id.checked_add(1)
                .expect("inspection subscription IDs exhausted"),
        );
        self.inner.entries.borrow_mut().insert(id, listener);
        let weak = Rc::downgrade(&self.inner);
        Subscription::new(move || {
            if let Some(inner) = weak.upgrade() {
                let removed = inner.entries.borrow_mut().remove(&id);
                drop(removed);
            }
        })
    }

    pub(crate) fn notify(&self, event: &E, snapshot: &Rc<Snapshot<C>>) -> StoreResult<()> {
        self.notify_with(|| Inspection::Transition {
            event,
            snapshot: snapshot.clone(),
        })
    }

    pub(crate) fn control(
        &self,
        action: ControlAction,
        snapshot: &Rc<Snapshot<C>>,
    ) -> StoreResult<()> {
        self.notify_with(|| Inspection::Control {
            action,
            snapshot: snapshot.clone(),
        })
    }

    fn notify_with<'a>(&self, inspection: impl Fn() -> Inspection<'a, C, E>) -> StoreResult<()> {
        let mut cursor = None;
        loop {
            let next = {
                let entries = self.inner.entries.borrow();
                let lower = cursor.map_or(Unbounded, Excluded);
                entries
                    .range((lower, Unbounded))
                    .next()
                    .map(|(id, listener)| (*id, listener.clone()))
            };
            let Some((id, listener)) = next else {
                return Ok(());
            };
            cursor = Some(id);
            listener(inspection())?;
        }
    }
}
