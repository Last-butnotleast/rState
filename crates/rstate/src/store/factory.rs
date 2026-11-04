use std::{fmt, rc::Rc};

use crate::{Enqueue, Store, StoreConfig, StoreResult, Update, transition::Reducer};

type Initializer<I, C> = dyn Fn(I) -> StoreResult<C>;

pub struct StoreFactory<I, C, E, M = ()> {
    initializer: Rc<Initializer<I, C>>,
    reducer: Rc<Reducer<C, E, M>>,
    history_limit: Option<usize>,
}

impl<I, C, E, M> Clone for StoreFactory<I, C, E, M> {
    fn clone(&self) -> Self {
        Self {
            initializer: self.initializer.clone(),
            reducer: self.reducer.clone(),
            history_limit: self.history_limit,
        }
    }
}

impl<I, C, E, M> fmt::Debug for StoreFactory<I, C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoreFactory").finish_non_exhaustive()
    }
}

impl<I, C, E, M> StoreFactory<I, C, E, M> {
    pub fn new(
        initializer: impl Fn(I) -> StoreResult<C> + 'static,
        reducer: impl Fn(&Rc<C>, &E, &mut Enqueue<C, E, M>) -> StoreResult<Update<C>> + 'static,
    ) -> Self {
        Self {
            initializer: Rc::new(initializer),
            reducer: Rc::new(reducer),
            history_limit: None,
        }
    }

    #[must_use]
    pub const fn with_history(mut self, limit: usize) -> Self {
        self.history_limit = Some(limit);
        self
    }
}

impl<I, C: 'static, E: 'static, M: 'static> StoreFactory<I, C, E, M> {
    pub fn create(&self, input: I) -> StoreResult<Store<C, E, M>> {
        self.config(input).map(Store::new)
    }

    pub(crate) fn config(&self, input: I) -> StoreResult<StoreConfig<C, E, M>> {
        let initial = Rc::new((self.initializer)(input)?);
        Ok(StoreConfig {
            initial,
            reducer: self.reducer.clone(),
            history_limit: self.history_limit,
        })
    }
}
