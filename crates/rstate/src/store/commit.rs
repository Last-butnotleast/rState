use std::rc::Rc;

use crate::{ControlAction, DispatchStatus, Effect, EffectScope, Snapshot, StoreResult};

use super::Store;

pub(super) enum Action<'a, E> {
    Event(&'a E),
    Control(ControlAction),
}

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub(super) fn commit(
        &self,
        snapshot: Rc<Snapshot<C>>,
        action: &Action<'_, E>,
        effects: Vec<Effect<C, E, M>>,
    ) -> StoreResult<()> {
        let previous = self.get_snapshot();
        #[expect(
            clippy::expect_used,
            reason = "exhausting monotonic commit revisions cannot preserve persistence ordering"
        )]
        let revision = self
            .inner
            .revision
            .get()
            .checked_add(1)
            .expect("store commit revisions exhausted");
        self.inner.revision.set(revision);
        let epoch = self.inner.persistence.as_ref().map(|state| state.epoch());
        self.inner.current.replace(snapshot.clone());

        if !Rc::ptr_eq(&previous, &snapshot) {
            self.inner
                .observers
                .publish()
                .map_err(|error| error.with_dispatch_status(DispatchStatus::Committed))?;
        }

        match action {
            Action::Event(event) => self.inner.inspectors.notify(event, &snapshot),
            Action::Control(action) => self.inner.inspectors.control(*action, &snapshot),
        }
        .map_err(|error| error.with_dispatch_status(DispatchStatus::Committed))?;

        if let Some((state, epoch)) = self.inner.persistence.as_ref().zip(epoch) {
            state.autosave(&snapshot.context, revision, epoch);
        }

        let scope = EffectScope::new(self.clone(), snapshot);
        for effect in effects {
            match effect {
                Effect::Emit(message) => self.inner.emissions.emit(&message),
                Effect::Run(effect) => effect(scope.clone()),
            }
            .map_err(|error| error.with_dispatch_status(DispatchStatus::Committed))?;
        }
        scope.complete();
        Ok(())
    }
}
