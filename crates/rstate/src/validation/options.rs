#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnknownPolicy {
    #[default]
    Reject,
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ValidationOptions {
    pub context: bool,
    pub events: bool,
    pub emitted: bool,
    pub unknown_events: UnknownPolicy,
    pub unknown_emitted: UnknownPolicy,
}

impl ValidationOptions {
    #[must_use]
    pub const fn with_context(mut self, enabled: bool) -> Self {
        self.context = enabled;
        self
    }

    #[must_use]
    pub const fn with_events(mut self, enabled: bool) -> Self {
        self.events = enabled;
        self
    }

    #[must_use]
    pub const fn with_emitted(mut self, enabled: bool) -> Self {
        self.emitted = enabled;
        self
    }

    #[must_use]
    pub const fn with_unknown_events(mut self, policy: UnknownPolicy) -> Self {
        self.unknown_events = policy;
        self
    }

    #[must_use]
    pub const fn with_unknown_emitted(mut self, policy: UnknownPolicy) -> Self {
        self.unknown_emitted = policy;
        self
    }
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            context: true,
            events: true,
            emitted: true,
            unknown_events: UnknownPolicy::Reject,
            unknown_emitted: UnknownPolicy::Reject,
        }
    }
}
