use std::rc::Rc;

use crate::persistence::PersistenceState;
use crate::{
    Persistence, PersistenceError, PersistenceStatus, StoreConfig, StoreError, StoreFactory,
    StoreResult, Subscription,
};

use super::Store;

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn new_persisted(
        config: StoreConfig<C, E, M>,
        persistence: Persistence<C>,
    ) -> StoreResult<Self> {
        let state = Rc::new(PersistenceState::new(persistence));
        let initial = state.load()?.unwrap_or_else(|| config.initial.clone());
        Ok(Self::from_parts(config, initial, Some(state), None))
    }

    pub fn save(&self) -> StoreResult<()> {
        self.persistence()?
            .save(self.get_snapshot().context.clone())
    }

    pub fn clear_storage(&self) -> StoreResult<()> {
        self.persistence()?.clear()
    }

    pub fn persistence_error(&self) -> Option<Rc<PersistenceError>> {
        self.inner
            .persistence
            .as_ref()
            .and_then(|state| state.error())
    }

    pub fn persistence_status(&self) -> Option<Rc<PersistenceStatus>> {
        self.inner.persistence.as_ref().map(|state| state.status())
    }

    pub fn subscribe_persistence(
        &self,
        listener: impl Fn(Rc<PersistenceStatus>) + 'static,
    ) -> StoreResult<Subscription> {
        Ok(self.persistence()?.subscribe(listener))
    }

    fn persistence(&self) -> StoreResult<&PersistenceState<C>> {
        self.inner
            .persistence
            .as_deref()
            .ok_or_else(|| StoreError::from_error(PersistenceError::NotConfigured))
    }
}

impl<I, C: 'static, E: 'static, M: 'static> StoreFactory<I, C, E, M> {
    pub fn create_persisted(
        &self,
        input: I,
        persistence: Persistence<C>,
    ) -> StoreResult<Store<C, E, M>> {
        Store::new_persisted(self.config(input)?, persistence)
    }
}
