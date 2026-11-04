use std::{cell::RefCell, fmt, rc::Rc};

use crate::{Snapshot, StoreConfig, StoreResult, Subscription};
use crate::{
    observation::Emissions, observation::Inspectors, observation::Observers, transition::Reducer,
};

pub struct Store<C, E, M = ()> {
    pub(crate) inner: Rc<StoreInner<C, E, M>>,
}

impl<C: fmt::Debug, E, M> fmt::Debug for Store<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self
            .inner
            .current
            .try_borrow()
            .map(|snapshot| snapshot.clone())
            .ok();
        let mut debug = f.debug_struct("Store");
        match current {
            Some(snapshot) => debug.field("current", &snapshot),
            None => debug.field("current", &"<borrowed>"),
        };
        debug.finish_non_exhaustive()
    }
}

pub struct StoreInner<C, E, M> {
    pub(crate) initial: Rc<Snapshot<C>>,
    pub(crate) current: RefCell<Rc<Snapshot<C>>>,
    pub(crate) reducer: Rc<Reducer<C, E, M>>,
    pub(crate) observers: Observers,
    pub(crate) inspectors: Inspectors<C, E>,
    pub(crate) emissions: Emissions<M>,
}

impl<C, E, M> Clone for Store<C, E, M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn new(config: StoreConfig<C, E, M>) -> Self {
        let snapshot = Rc::new(Snapshot::new(config.initial));
        Self {
            inner: Rc::new(StoreInner {
                initial: snapshot.clone(),
                current: RefCell::new(snapshot),
                reducer: config.reducer,
                observers: Observers::default(),
                inspectors: Inspectors::default(),
                emissions: Emissions::default(),
            }),
        }
    }

    pub fn get_snapshot(&self) -> Rc<Snapshot<C>> {
        self.inner.current.borrow().clone()
    }

    pub fn get_initial_snapshot(&self) -> Rc<Snapshot<C>> {
        self.inner.initial.clone()
    }

    pub fn subscribe(
        &self,
        listener: impl Fn(Rc<Snapshot<C>>) -> StoreResult<()> + 'static,
    ) -> Subscription {
        let weak = Rc::downgrade(&self.inner);
        self.inner.observers.subscribe(move || {
            if let Some(inner) = weak.upgrade() {
                let snapshot = inner.current.borrow().clone();
                listener(snapshot)?;
            }
            Ok(())
        })
    }

    pub fn on(
        &self,
        filter: impl Fn(&M) -> bool + 'static,
        listener: impl Fn(Rc<M>) -> StoreResult<()> + 'static,
    ) -> Subscription {
        self.inner.emissions.on(filter, listener)
    }

    pub fn on_any(&self, listener: impl Fn(Rc<M>) -> StoreResult<()> + 'static) -> Subscription {
        self.inner.emissions.on_any(listener)
    }
}
