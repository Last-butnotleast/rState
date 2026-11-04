use std::rc::Rc;

use crate::{Snapshot, StoreResult, TransitionResult, transition::reduce};

use super::{Store, commit::Action};

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn transition(
        &self,
        snapshot: Rc<Snapshot<C>>,
        event: E,
    ) -> StoreResult<TransitionResult<C, E, M>> {
        let result = self.evaluate(&snapshot, &event);
        drop(snapshot);
        drop(event);
        result
    }

    pub fn can(&self, event: E) -> StoreResult<bool> {
        self.transition(self.get_snapshot(), event)
            .map(|result| result.allowed)
    }

    pub fn send(&self, event: E) -> StoreResult<()> {
        let previous = self.get_snapshot();
        let result = self.evaluate(&previous, &event)?;
        self.commit(result.snapshot, &Action::Event(&event), result.effects)?;
        drop(event);
        Ok(())
    }

    fn evaluate(
        &self,
        snapshot: &Rc<Snapshot<C>>,
        event: &E,
    ) -> StoreResult<TransitionResult<C, E, M>> {
        let mut result = reduce(self.inner.reducer.as_ref(), snapshot.clone(), event)?;
        if self.inner.history_limit.is_some() {
            result.snapshot =
                Rc::new(snapshot.record(result.snapshot.context.clone(), self.inner.history_limit));
        }
        Ok(result)
    }
}
