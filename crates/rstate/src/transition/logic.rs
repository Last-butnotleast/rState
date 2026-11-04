use std::{fmt, rc::Rc};

use crate::{
    Enqueue, Schemas, Snapshot, StoreError, StoreResult, TransitionResult, Update,
    ValidationOptions,
};

use super::{Reducer, evaluate};

pub struct Transition<C, E, M = ()> {
    reducer: Rc<Reducer<C, E, M>>,
    schemas: Schemas<C, E, M>,
    validation: Option<ValidationOptions>,
}

impl<C, E, M> Clone for Transition<C, E, M> {
    fn clone(&self) -> Self {
        Self {
            reducer: self.reducer.clone(),
            schemas: self.schemas.clone(),
            validation: self.validation,
        }
    }
}

impl<C, E, M> fmt::Debug for Transition<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transition").finish_non_exhaustive()
    }
}

impl<C, E, M> Transition<C, E, M> {
    pub fn new(
        reducer: impl Fn(&Rc<C>, &E, &mut Enqueue<C, E, M>) -> StoreResult<Update<C>> + 'static,
    ) -> Self {
        Self {
            reducer: Rc::new(reducer),
            schemas: Schemas::default(),
            validation: None,
        }
    }

    #[must_use]
    pub fn with_schemas(mut self, schemas: Schemas<C, E, M>) -> Self {
        self.schemas = schemas;
        self
    }

    #[must_use]
    pub const fn with_validation(mut self, options: ValidationOptions) -> Self {
        self.validation = Some(options);
        self
    }

    pub const fn schemas(&self) -> &Schemas<C, E, M> {
        &self.schemas
    }

    pub fn transition(
        &self,
        snapshot: &Rc<Snapshot<C>>,
        event: E,
    ) -> StoreResult<TransitionResult<C, E, M>> {
        let result = evaluate(
            self.reducer.as_ref(),
            snapshot,
            &event,
            &self.schemas,
            self.validation,
        )
        .map_err(StoreError::without_dispatch_status);
        drop(event);
        result
    }
}
