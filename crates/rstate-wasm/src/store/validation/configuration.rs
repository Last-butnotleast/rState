use rstate::{UnknownPolicy, ValidationOptions};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    #[serde(default = "enabled")]
    pub runtime: bool,
    #[serde(default)]
    pub initial_count: i64,
    pub history_limit: Option<usize>,
    #[serde(default)]
    pub no_schemas: bool,
    #[serde(flatten)]
    checks: Checks,
    #[serde(default)]
    unknown_events: Policy,
    #[serde(default)]
    unknown_emitted: Policy,
    #[serde(default)]
    pub missing_receipt: bool,
    pub persistence: Option<PersistenceConfiguration>,
}

#[derive(Deserialize)]
struct Checks {
    #[serde(default = "enabled")]
    context: bool,
    #[serde(default = "enabled")]
    events: bool,
    #[serde(default = "enabled")]
    emitted: bool,
}

#[derive(Deserialize)]
pub struct PersistenceConfiguration {
    pub name: String,
    pub version: u64,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Policy {
    #[default]
    Throw,
    Ignore,
}

impl Configuration {
    pub fn options(&self) -> ValidationOptions {
        ValidationOptions::default()
            .with_context(self.checks.context)
            .with_events(self.checks.events)
            .with_emitted(self.checks.emitted)
            .with_unknown_events(self.unknown_events.into())
            .with_unknown_emitted(self.unknown_emitted.into())
    }
}

impl From<Policy> for UnknownPolicy {
    fn from(policy: Policy) -> Self {
        match policy {
            Policy::Throw => Self::Reject,
            Policy::Ignore => Self::Ignore,
        }
    }
}

const fn enabled() -> bool {
    true
}
