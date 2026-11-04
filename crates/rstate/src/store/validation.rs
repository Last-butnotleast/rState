use std::rc::Rc;

use crate::persistence::PersistenceState;
use crate::{
    Persistence, StoreConfig, StoreFactory, StoreResult, ValidationOptions, ValidationStage,
};

use super::Store;

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn new_validated(
        config: StoreConfig<C, E, M>,
        options: ValidationOptions,
    ) -> StoreResult<Self> {
        options.check_context(
            config.schemas.context(),
            &config.initial,
            ValidationStage::InitialContext,
        )?;
        let initial = config.initial.clone();
        Ok(Self::from_parts(config, initial, None, Some(options)))
    }

    pub fn new_persisted_validated(
        config: StoreConfig<C, E, M>,
        persistence: Persistence<C>,
        options: ValidationOptions,
    ) -> StoreResult<Self> {
        options.check_context(
            config.schemas.context(),
            &config.initial,
            ValidationStage::InitialContext,
        )?;
        let state = Rc::new(PersistenceState::new(persistence));
        let initial = if let Some(initial) = state.load()? {
            options.check_context(
                config.schemas.context(),
                &initial,
                ValidationStage::InitialContext,
            )?;
            initial
        } else {
            config.initial.clone()
        };
        Ok(Self::from_parts(
            config,
            initial,
            Some(state),
            Some(options),
        ))
    }

    pub(super) fn validate_context(&self, context: &C, stage: ValidationStage) -> StoreResult<()> {
        if let Some(options) = self.inner.validation {
            options.check_context(self.inner.schemas.context(), context, stage)?;
        }
        Ok(())
    }
}

impl<I, C: 'static, E: 'static, M: 'static> StoreFactory<I, C, E, M> {
    pub fn create_validated(
        &self,
        input: I,
        options: ValidationOptions,
    ) -> StoreResult<Store<C, E, M>> {
        Store::new_validated(self.config(input)?, options)
    }

    pub fn create_persisted_validated(
        &self,
        input: I,
        persistence: Persistence<C>,
        options: ValidationOptions,
    ) -> StoreResult<Store<C, E, M>> {
        Store::new_persisted_validated(self.config(input)?, persistence, options)
    }
}
