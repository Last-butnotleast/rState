use std::rc::Rc;

use crate::{EffectScope, StoreResult};

type EffectCallback<C, E, M> = dyn FnOnce(EffectScope<C, E, M>) -> StoreResult<()>;

pub enum Effect<C, E, M = ()> {
    Emit(Rc<M>),
    Run(Box<EffectCallback<C, E, M>>),
}

pub struct Enqueue<C, E, M = ()> {
    pub(crate) events: Vec<E>,
    pub(crate) effects: Vec<Effect<C, E, M>>,
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
