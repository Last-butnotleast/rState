use std::{fmt, rc::Rc};

use crate::{Enqueue, StoreResult, Update};

pub type Reducer<C, E, M> = dyn Fn(&Rc<C>, &E, &mut Enqueue<C, E, M>) -> StoreResult<Update<C>>;

pub struct StoreConfig<C, E, M = ()> {
    pub(crate) initial: Rc<C>,
    pub(crate) reducer: Rc<Reducer<C, E, M>>,
    pub(crate) history_limit: Option<usize>,
}

impl<C: fmt::Debug, E, M> fmt::Debug for StoreConfig<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoreConfig")
            .field("initial", &self.initial)
            .finish_non_exhaustive()
    }
}

impl<C, E, M> StoreConfig<C, E, M> {
    pub fn new(
        initial: C,
        reducer: impl Fn(&Rc<C>, &E, &mut Enqueue<C, E, M>) -> StoreResult<Update<C>> + 'static,
    ) -> Self {
        Self {
            initial: Rc::new(initial),
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
