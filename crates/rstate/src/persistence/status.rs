use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::Subscription;

use super::PersistenceError;

mod listeners;

use listeners::Listeners;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PersistenceActivity {
    AutomaticSave,
    Save,
    Clear,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum PersistenceOutcome {
    Succeeded,
    Superseded,
    Failed(Rc<PersistenceError>),
}

#[derive(Debug, Default)]
pub struct PersistenceStatus {
    sequence: u64,
    activity: Option<PersistenceActivity>,
    outcome: Option<PersistenceOutcome>,
    error: Option<Rc<PersistenceError>>,
}

impl PersistenceStatus {
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn last_activity(&self) -> Option<PersistenceActivity> {
        self.activity
    }

    pub const fn outcome(&self) -> Option<&PersistenceOutcome> {
        self.outcome.as_ref()
    }

    pub const fn error(&self) -> Option<&Rc<PersistenceError>> {
        self.error.as_ref()
    }
}

pub struct StatusState {
    current: RefCell<Rc<PersistenceStatus>>,
    dirty: Cell<bool>,
    listeners: Listeners,
}

impl Default for StatusState {
    fn default() -> Self {
        Self {
            current: RefCell::new(Rc::default()),
            dirty: Cell::new(false),
            listeners: Listeners::default(),
        }
    }
}

impl StatusState {
    pub(super) fn get(&self) -> Rc<PersistenceStatus> {
        self.current.borrow().clone()
    }

    pub(super) fn subscribe(
        &self,
        listener: impl Fn(Rc<PersistenceStatus>) + 'static,
    ) -> Subscription {
        self.listeners.subscribe(self.get().sequence(), listener)
    }

    pub(super) fn record(
        &self,
        activity: PersistenceActivity,
        outcome: PersistenceOutcome,
        error: Option<Rc<PersistenceError>>,
    ) -> Rc<PersistenceStatus> {
        #[expect(
            clippy::expect_used,
            reason = "exhausting persistence status sequences cannot preserve observation order"
        )]
        let sequence = self
            .current
            .borrow()
            .sequence()
            .checked_add(1)
            .expect("persistence status sequences exhausted");
        self.dirty.set(true);
        self.current.replace(Rc::new(PersistenceStatus {
            sequence,
            activity: Some(activity),
            outcome: Some(outcome),
            error,
        }))
    }

    pub(super) fn publish(&self) {
        if self.dirty.replace(false) {
            self.listeners.publish(|| self.get());
        }
    }
}
