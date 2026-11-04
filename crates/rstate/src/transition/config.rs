use std::rc::Rc;

use crate::{Enqueue, StoreResult, Update};

pub(crate) type Reducer<C, E, M> =
    dyn Fn(&Rc<C>, &E, &mut Enqueue<C, E, M>) -> StoreResult<Update<C>>;

pub struct StoreConfig<C, E, M = ()> {
    pub(crate) initial: Rc<C>,
    pub(crate) reducer: Rc<Reducer<C, E, M>>,
}

impl<C, E, M> StoreConfig<C, E, M> {
    pub fn new(
        initial: C,
        reducer: impl Fn(&Rc<C>, &E, &mut Enqueue<C, E, M>) -> StoreResult<Update<C>> + 'static,
    ) -> Self {
        Self {
            initial: Rc::new(initial),
            reducer: Rc::new(reducer),
        }
    }
}
