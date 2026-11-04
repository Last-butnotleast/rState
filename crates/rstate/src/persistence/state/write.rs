use std::rc::Rc;

use crate::{StoreError, StoreResult};

use super::{
    Flag, PersistenceActivity, PersistenceError, PersistenceOperation, PersistenceState, Save,
};

impl<C> PersistenceState<C> {
    pub(crate) fn autosave(&self, context: &Rc<C>, revision: u64, epoch: u64) {
        let request = Save {
            context: context.clone(),
            revision,
            epoch,
        };
        if !self.current(&request) || self.scheduled.get().is_some_and(|saved| saved >= revision) {
            return;
        }
        self.scheduled.set(Some(revision));
        if self.busy.get() {
            let previous = self.pending.replace(Some(request));
            drop(previous);
            return;
        }
        let busy = Flag::enter(&self.busy);
        let result = self.write(&request);
        self.report(&result, true);
        let drained = self.drain();
        drop(busy);
        drop(drained);
        drop(result);
        self.publish_status();
    }

    pub(crate) fn save(&self, context: Rc<C>) -> StoreResult<()> {
        if self.busy.get() {
            return self.reject(PersistenceActivity::Save, PersistenceOperation::Save);
        }
        let request = Save {
            context,
            revision: self.revision.get(),
            epoch: self.epoch.get(),
        };
        let busy = Flag::enter(&self.busy);
        let (result, retired) = self.save_current(&request);
        let previous = self.complete_status(PersistenceActivity::Save, &result);
        drop(busy);
        drop(previous);
        drop(retired);
        drop(request);
        self.publish_status();
        result.map_err(StoreError::from_error)
    }

    fn save_current(
        &self,
        request: &Save<C>,
    ) -> (Result<(), PersistenceError>, Option<PersistenceError>) {
        self.scheduled.set(Some(request.revision));
        let result = self.write(request);
        self.report(&result, false);
        let drained = self.drain();
        let wrote = match result {
            Ok(wrote) => wrote,
            Err(error) => return (Err(error), drained.err()),
        };
        let saved = match drained {
            Ok(saved) => saved.or_else(|| wrote.then_some((request.revision, request.epoch))),
            Err(error) => return (Err(error), None),
        };
        let outcome = if saved == Some((self.revision.get(), self.epoch.get())) {
            Ok(())
        } else {
            Err(PersistenceError::Superseded)
        };
        (outcome, None)
    }

    pub(crate) fn clear(&self) -> StoreResult<()> {
        if self.io_active.get() {
            return self.reject(PersistenceActivity::Clear, PersistenceOperation::Clear);
        }
        let owner = !self.busy.get();
        let busy = owner.then(|| Flag::enter(&self.busy));
        let (result, retired) = self.clear_current(owner);
        let previous = self.complete_status(PersistenceActivity::Clear, &result);
        drop(busy);
        drop(previous);
        drop(retired);
        self.publish_status();
        result.map_err(StoreError::from_error)
    }

    fn clear_current(
        &self,
        owner: bool,
    ) -> (Result<(), PersistenceError>, Option<PersistenceError>) {
        let Some(epoch) = self.epoch.get().checked_add(1) else {
            return (Err(PersistenceError::EpochExhausted), None);
        };
        self.epoch.set(epoch);
        self.scheduled.set(None);
        let pending = self.pending.take();
        drop(pending);
        let result = {
            let _io = Flag::enter(&self.io_active);
            self.config
                .storage
                .remove(&self.config.name)
                .map(|()| true)
                .map_err(|error| PersistenceError::operation(PersistenceOperation::Clear, error))
        };
        self.report(&result, false);
        let retired = owner.then(|| self.drain().err()).flatten();
        (result.map(|_| ()), retired)
    }

    fn reject(
        &self,
        activity: PersistenceActivity,
        operation: PersistenceOperation,
    ) -> StoreResult<()> {
        let result = Err(PersistenceError::Reentrant { operation });
        drop(self.complete_status(activity, &result));
        self.publish_status();
        result.map_err(StoreError::from_error)
    }

    fn current(&self, request: &Save<C>) -> bool {
        request.revision == self.revision.get() && request.epoch == self.epoch.get()
    }

    fn write(&self, request: &Save<C>) -> Result<bool, PersistenceError> {
        if !self.current(request) {
            return Ok(false);
        }
        let bytes = self
            .config
            .codec
            .encode(&request.context, self.config.version)
            .map_err(|error| PersistenceError::operation(PersistenceOperation::Encode, error))?;
        if !self.current(request) {
            return Ok(false);
        }
        let _io = Flag::enter(&self.io_active);
        self.config
            .storage
            .write(&self.config.name, &bytes)
            .map(|()| true)
            .map_err(|error| PersistenceError::operation(PersistenceOperation::Save, error))
    }

    fn drain(&self) -> Result<Option<(u64, u64)>, PersistenceError> {
        let mut saved = None;
        let mut failure = None;
        loop {
            let Some(request) = self.pending.take() else {
                break;
            };
            let result = self.write(&request);
            self.report(&result, true);
            match result {
                Ok(true) => saved = Some((request.revision, request.epoch)),
                Ok(false) => {}
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        failure.map_or(Ok(saved), Err)
    }
}
