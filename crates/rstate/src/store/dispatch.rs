use std::rc::Rc;

use crate::{Effect, EffectScope, Snapshot, StoreResult, TransitionResult, transition::reduce};

use super::Store;

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn transition(
        &self,
        snapshot: Rc<Snapshot<C>>,
        event: E,
    ) -> StoreResult<TransitionResult<C, E, M>> {
        reduce(self.inner.reducer.as_ref(), snapshot, event)
    }

    pub fn can(&self, event: E) -> StoreResult<bool> {
        self.transition(self.get_snapshot(), event)
            .map(|result| result.allowed)
    }

    pub fn send(&self, event: E) -> StoreResult<()> {
        let previous = self.get_snapshot();
        let result = self.transition(previous.clone(), event)?;
        self.inner.current.replace(result.snapshot.clone());

        if !Rc::ptr_eq(&previous, &result.snapshot) {
            self.inner.observers.publish()?;
        }

        let scope = EffectScope::new(self.clone(), result.snapshot);
        for effect in result.effects {
            match effect {
                Effect::Emit(message) => self.inner.emissions.emit(message)?,
                Effect::Run(effect) => effect(scope.clone())?,
            }
        }
        scope.complete();
        Ok(())
    }
}
