use std::rc::Rc;

use crate::{Effect, Snapshot, StoreResult, TransitionResult, ValidationStage, transition::reduce};

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
        match self.transition(self.get_snapshot(), event) {
            Ok(result) => Ok(result.allowed),
            Err(error) if error.validation_error().is_some() => Ok(false),
            Err(error) => Err(error),
        }
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
        if let Some(options) = self.inner.validation {
            options.check_event(self.inner.schemas.events(), event)?;
        }
        let mut result = reduce(self.inner.reducer.as_ref(), snapshot.clone(), event)?;
        if let Some(options) = self.inner.validation {
            options.check_context(
                self.inner.schemas.context(),
                &result.snapshot.context,
                ValidationStage::Context,
            )?;
            for effect in &result.effects {
                if let Effect::Emit(message) = effect {
                    options.check_emitted(self.inner.schemas.emitted(), message.as_ref())?;
                }
            }
        }
        if self.inner.history_limit.is_some() {
            result.snapshot =
                Rc::new(snapshot.record(result.snapshot.context.clone(), self.inner.history_limit));
        }
        Ok(result)
    }
}
