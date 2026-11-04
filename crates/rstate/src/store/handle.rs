use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use crate::persistence::PersistenceState;
use crate::{Schemas, Snapshot, StoreConfig, StoreResult, Subscription, ValidationOptions};
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
    pub(crate) defaults: Rc<C>,
    pub(crate) initial: Rc<Snapshot<C>>,
    pub(crate) current: RefCell<Rc<Snapshot<C>>>,
    pub(crate) reducer: Rc<Reducer<C, E, M>>,
    pub(crate) observers: Observers,
    pub(crate) inspectors: Inspectors<C, E>,
    pub(crate) emissions: Emissions<M>,
    pub(crate) history_limit: Option<usize>,
    pub(crate) revision: Rc<Cell<u64>>,
    pub(crate) persistence: Option<Rc<PersistenceState<C>>>,
    pub(crate) schemas: Schemas<C, E, M>,
    pub(crate) validation: Option<ValidationOptions>,
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
        let initial = config.initial.clone();
        Self::from_parts(config, initial, None, None)
    }

    pub(crate) fn from_parts(
        config: StoreConfig<C, E, M>,
        initial: Rc<C>,
        persistence: Option<Rc<PersistenceState<C>>>,
        validation: Option<ValidationOptions>,
    ) -> Self {
        let snapshot = Rc::new(Snapshot::initial(initial, config.history_limit));
        let revision = persistence
            .as_ref()
            .map_or_else(|| Rc::new(Cell::new(0)), |state| state.revision());
        Self {
            inner: Rc::new(StoreInner {
                defaults: config.initial,
                initial: snapshot.clone(),
                current: RefCell::new(snapshot),
                reducer: config.reducer,
                observers: Observers::default(),
                inspectors: Inspectors::default(),
                emissions: Emissions::default(),
                history_limit: config.history_limit,
                revision,
                persistence,
                schemas: config.schemas,
                validation,
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

    pub fn schemas(&self) -> &Schemas<C, E, M> {
        &self.inner.schemas
    }
}
