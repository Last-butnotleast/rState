use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use crate::{StoreError, StoreResult, Subscription};

use super::{
    Persistence, PersistenceActivity, PersistenceError, PersistenceOperation, PersistenceOutcome,
    PersistenceStatus, status::StatusState,
};

mod write;

struct Save<C> {
    context: Rc<C>,
    revision: u64,
    epoch: u64,
}

pub struct PersistenceState<C> {
    config: Persistence<C>,
    revision: Rc<Cell<u64>>,
    epoch: Cell<u64>,
    scheduled: Cell<Option<u64>>,
    busy: Cell<bool>,
    io_active: Cell<bool>,
    pending: RefCell<Option<Save<C>>>,
    error: RefCell<Option<Rc<PersistenceError>>>,
    status: StatusState,
}

impl<C> fmt::Debug for PersistenceState<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistenceState")
            .field("configuration", &self.config)
            .field("revision", &self.revision.get())
            .field("epoch", &self.epoch.get())
            .field("busy", &self.busy.get())
            .finish_non_exhaustive()
    }
}

impl<C> PersistenceState<C> {
    pub(crate) fn new(config: Persistence<C>) -> Self {
        Self {
            config,
            revision: Rc::new(Cell::new(0)),
            epoch: Cell::new(0),
            scheduled: Cell::new(None),
            busy: Cell::new(false),
            io_active: Cell::new(false),
            pending: RefCell::new(None),
            error: RefCell::new(None),
            status: StatusState::default(),
        }
    }

    pub(crate) fn revision(&self) -> Rc<Cell<u64>> {
        self.revision.clone()
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.epoch.get()
    }

    pub(crate) fn error(&self) -> Option<Rc<PersistenceError>> {
        self.error.borrow().clone()
    }

    pub(crate) fn status(&self) -> Rc<PersistenceStatus> {
        self.status.get()
    }

    pub(crate) fn subscribe(
        &self,
        listener: impl Fn(Rc<PersistenceStatus>) + 'static,
    ) -> Subscription {
        self.status.subscribe(listener)
    }

    pub(crate) fn load(&self) -> StoreResult<Option<Rc<C>>> {
        let bytes = self
            .config
            .storage
            .read(&self.config.name)
            .map_err(|error| Self::failure(PersistenceOperation::Load, error))?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let persisted = self
            .config
            .codec
            .decode(&bytes)
            .map_err(|error| Self::failure(PersistenceOperation::Decode, error))?;
        if persisted.version != self.config.version {
            return Err(StoreError::from_error(PersistenceError::VersionMismatch {
                expected: self.config.version,
                actual: persisted.version,
            }));
        }
        Ok(Some(Rc::new(persisted.context)))
    }

    fn failure(operation: PersistenceOperation, error: StoreError) -> StoreError {
        StoreError::from_error(PersistenceError::operation(operation, error))
    }

    fn report(&self, result: &Result<bool, PersistenceError>, automatic: bool) {
        match result {
            Ok(false) => {}
            Ok(true) => {
                let previous = self.error.take();
                if automatic {
                    drop(self.record_status(
                        PersistenceActivity::AutomaticSave,
                        PersistenceOutcome::Succeeded,
                    ));
                }
                drop(previous);
            }
            Err(error) => {
                let error = Rc::new(error.clone());
                let previous = self.error.replace(Some(error.clone()));
                if automatic {
                    drop(self.record_status(
                        PersistenceActivity::AutomaticSave,
                        PersistenceOutcome::Failed(error.clone()),
                    ));
                }
                drop(previous);
                if let Some(listener) = &self.config.on_error {
                    listener(&error);
                }
            }
        }
    }

    fn record_status(
        &self,
        activity: PersistenceActivity,
        outcome: PersistenceOutcome,
    ) -> Rc<PersistenceStatus> {
        self.status.record(activity, outcome, self.error())
    }

    fn complete_status(
        &self,
        activity: PersistenceActivity,
        result: &Result<(), PersistenceError>,
    ) -> Rc<PersistenceStatus> {
        let outcome = match result {
            Ok(()) => PersistenceOutcome::Succeeded,
            Err(PersistenceError::Superseded) => PersistenceOutcome::Superseded,
            Err(error) => PersistenceOutcome::Failed(Rc::new(error.clone())),
        };
        self.record_status(activity, outcome)
    }

    fn publish_status(&self) {
        if !self.busy.get() && !self.io_active.get() {
            self.status.publish();
        }
    }
}

struct Flag<'a>(&'a Cell<bool>);

impl<'a> Flag<'a> {
    fn enter(cell: &'a Cell<bool>) -> Self {
        cell.set(true);
        Self(cell)
    }
}

impl Drop for Flag<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
