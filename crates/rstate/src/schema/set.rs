use std::fmt;

use crate::Schema;

pub struct Schemas<C, E, M = ()> {
    context: Option<Schema<C>>,
    events: Option<Schema<E>>,
    emitted: Option<Schema<M>>,
}

impl<C, E, M> Default for Schemas<C, E, M> {
    fn default() -> Self {
        Self {
            context: None,
            events: None,
            emitted: None,
        }
    }
}

impl<C, E, M> Clone for Schemas<C, E, M> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            events: self.events.clone(),
            emitted: self.emitted.clone(),
        }
    }
}

impl<C, E, M> fmt::Debug for Schemas<C, E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schemas")
            .field("context", &self.context)
            .field("events", &self.events)
            .field("emitted", &self.emitted)
            .finish()
    }
}

impl<C, E, M> Schemas<C, E, M> {
    #[must_use]
    pub fn with_context(mut self, schema: Schema<C>) -> Self {
        self.context = Some(schema);
        self
    }

    #[must_use]
    pub fn with_events(mut self, schema: Schema<E>) -> Self {
        self.events = Some(schema);
        self
    }

    #[must_use]
    pub fn with_emitted(mut self, schema: Schema<M>) -> Self {
        self.emitted = Some(schema);
        self
    }

    pub const fn context(&self) -> Option<&Schema<C>> {
        self.context.as_ref()
    }

    pub const fn events(&self) -> Option<&Schema<E>> {
        self.events.as_ref()
    }

    pub const fn emitted(&self) -> Option<&Schema<M>> {
        self.emitted.as_ref()
    }
}
