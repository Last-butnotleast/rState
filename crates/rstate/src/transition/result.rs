use std::{fmt, rc::Rc};

use crate::{Effect, Snapshot};

#[non_exhaustive]
pub struct TransitionResult<C, E, M = ()> {
    pub snapshot: Rc<Snapshot<C>>,
    pub effects: Vec<Effect<C, E, M>>,
    pub allowed: bool,
}

impl<C: fmt::Debug, E, M> fmt::Debug for TransitionResult<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitionResult")
            .field("snapshot", &self.snapshot)
            .field("effects", &self.effects)
            .field("allowed", &self.allowed)
            .finish()
    }
}
