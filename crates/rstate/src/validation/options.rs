#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnknownPolicy {
    #[default]
    Reject,
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationOptions {
    pub context: bool,
    pub events: bool,
    pub emitted: bool,
    pub unknown_events: UnknownPolicy,
    pub unknown_emitted: UnknownPolicy,
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
