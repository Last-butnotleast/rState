use std::rc::Rc;

use crate::{
    DispatchStatus, Snapshot, StoreError, StoreResult, TransitionResult, transition::evaluate,
};

use super::{Store, commit::Action};

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn transition(
        &self,
        snapshot: Rc<Snapshot<C>>,
        event: E,
    ) -> StoreResult<TransitionResult<C, E, M>> {
        let result = self
            .evaluate(&snapshot, &event)
            .map_err(StoreError::without_dispatch_status);
        drop(snapshot);
        drop(event);
        result
    }

    pub fn can(&self, event: E) -> StoreResult<bool> {
        match self.transition(self.get_snapshot(), event) {
            Ok(result) => Ok(result.allowed),
            Err(error) if error.validation_error().is_some() => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn send(&self, event: E) -> StoreResult<()> {
        let previous = self.get_snapshot();
        let result = self
            .evaluate(&previous, &event)
            .map_err(|error| error.with_dispatch_status(DispatchStatus::NotCommitted))?;
        self.commit(result.snapshot, &Action::Event(&event), result.effects)?;
        drop(event);
        Ok(())
    }

    fn evaluate(
        &self,
        snapshot: &Rc<Snapshot<C>>,
        event: &E,
    ) -> StoreResult<TransitionResult<C, E, M>> {
        let mut result = evaluate(
            self.inner.reducer.as_ref(),
            snapshot,
            event,
            &self.inner.schemas,
            self.inner.validation,
        )?;
        if self.inner.history_limit.is_some() {
            result.snapshot =
                Rc::new(snapshot.record(result.snapshot.context.clone(), self.inner.history_limit));
        }
        Ok(result)
    }
}
