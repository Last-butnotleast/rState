use std::{fmt, rc::Rc};

use crate::{EffectScope, StoreResult};

type EffectCallback<C, E, M> = dyn FnOnce(EffectScope<C, E, M>) -> StoreResult<()>;

pub enum Effect<C, E, M = ()> {
    Emit(Rc<M>),
    Run(Box<EffectCallback<C, E, M>>),
}

impl<C, E, M> fmt::Debug for Effect<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Emit(_) => f.write_str("Emit(..)"),
            Self::Run(_) => f.write_str("Run(..)"),
        }
    }
}

pub struct Enqueue<C, E, M = ()> {
    pub(crate) events: Vec<E>,
    pub(crate) effects: Vec<Effect<C, E, M>>,
}

impl<C, E, M> fmt::Debug for Enqueue<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Enqueue")
            .field("queued_events", &self.events.len())
            .field("effects", &self.effects)
            .finish()
    }
}

impl<C, E, M> Default for Enqueue<C, E, M> {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            effects: Vec::new(),
        }
    }
}

impl<C, E, M> Enqueue<C, E, M> {
    pub fn trigger(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn emit(&mut self, message: M) {
        self.effects.push(Effect::Emit(Rc::new(message)));
    }

    pub fn effect(
        &mut self,
        effect: impl FnOnce(EffectScope<C, E, M>) -> StoreResult<()> + 'static,
    ) {
        self.effects.push(Effect::Run(Box::new(effect)));
    }
}
