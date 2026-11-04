use crate::{Inspection, StoreResult, Subscription};

use super::Store;

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn inspect(
        &self,
        listener: impl for<'a> Fn(Inspection<'a, C, E>) -> StoreResult<()> + 'static,
    ) -> StoreResult<Subscription> {
        self.inner.inspectors.inspect(self.get_snapshot(), listener)
    }
}
