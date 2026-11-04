use std::rc::Rc;

use crate::{StoreError, StoreResult};

use super::{Flag, PersistenceError, PersistenceOperation, PersistenceState, Save};

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
        let _busy = Flag::enter(&self.busy);
        let result = self.write(&request);
        self.report(&result);
        drop(self.drain());
    }

    pub(crate) fn save(&self, context: Rc<C>) -> StoreResult<()> {
        if self.busy.get() {
            return Err(StoreError::from_error(PersistenceError::Reentrant {
                operation: PersistenceOperation::Save,
            }));
        }
        let request = Save {
            context,
            revision: self.revision.get(),
            epoch: self.epoch.get(),
        };
        self.scheduled.set(Some(request.revision));
        let _busy = Flag::enter(&self.busy);
        let result = self.write(&request);
        self.report(&result);
        let drained = self.drain();
        let wrote = result.map_err(StoreError::from_error)?;
        let saved = drained
            .map_err(StoreError::from_error)?
            .or_else(|| wrote.then_some((request.revision, request.epoch)));
        if saved == Some((self.revision.get(), self.epoch.get())) {
            Ok(())
        } else {
            Err(StoreError::from_error(PersistenceError::Superseded))
        }
    }

    pub(crate) fn clear(&self) -> StoreResult<()> {
        if self.io_active.get() {
            return Err(StoreError::from_error(PersistenceError::Reentrant {
                operation: PersistenceOperation::Clear,
            }));
        }
        let owner = !self.busy.get();
        let _busy = owner.then(|| Flag::enter(&self.busy));
        let epoch = self
            .epoch
            .get()
            .checked_add(1)
            .ok_or_else(|| StoreError::from_error(PersistenceError::EpochExhausted))?;
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
        self.report(&result);
        if owner {
            drop(self.drain());
        }
        result.map(|_| ()).map_err(StoreError::from_error)
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
            self.report(&result);
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
