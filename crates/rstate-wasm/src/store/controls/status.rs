use js_sys::Function;
use rstate::{PersistenceActivity, PersistenceOutcome, PersistenceStatus};
use serde_json::{Value, json};

use super::storage::original_message;
use crate::scenarios::Fixture;
use crate::store::handle::invoke;

pub(in crate::store) fn read(store: &Fixture) -> String {
    store
        .persistence_status()
        .as_deref()
        .map_or(Value::Null, value)
        .to_string()
}

pub(in crate::store) fn notify(
    status: &PersistenceStatus,
    callback: &Function,
    diagnostics: &Function,
) {
    if let Err(failure) = invoke(callback, &value(status)) {
        let _ = invoke(diagnostics, &json!(failure.to_string()));
    }
}

fn value(status: &PersistenceStatus) -> Value {
    let activity = status.last_activity().map(|activity| match activity {
        PersistenceActivity::AutomaticSave => "automaticSave",
        PersistenceActivity::Save => "save",
        PersistenceActivity::Clear => "clear",
        _ => "unknown",
    });
    let outcome = status.outcome().map(|outcome| match outcome {
        PersistenceOutcome::Succeeded => "succeeded",
        PersistenceOutcome::Superseded => "superseded",
        PersistenceOutcome::Failed(_) => "failed",
        _ => "unknown",
    });
    let error = status
        .error()
        .map(|failure| original_message(failure.as_ref()));
    json!({
        "sequence": status.sequence(),
        "activity": activity,
        "outcome": outcome,
        "error": error,
    })
}
