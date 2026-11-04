use std::rc::Rc;

use crate::{Effect, Snapshot};

pub struct TransitionResult<C, E, M = ()> {
    pub snapshot: Rc<Snapshot<C>>,
    pub effects: Vec<Effect<C, E, M>>,
    pub allowed: bool,
}
