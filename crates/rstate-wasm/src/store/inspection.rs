use std::cell::RefCell;

use js_sys::Function;
use rstate::{ControlAction, Inspection, StoreError, StoreResult};
use serde_json::json;

use super::{handle::invoke, identity::Identities};
use crate::scenarios::{Context, Event};

pub(super) fn notify(
    record: &Inspection<'_, Context, Event>,
    identities: &RefCell<Identities>,
    callback: &Function,
) -> StoreResult {
    let (event, snapshot) = match record {
        Inspection::Initial { snapshot } => (json!({ "type": "@xstate.init" }), snapshot),
        Inspection::Transition { event, snapshot } => (
            serde_json::to_value(event).map_err(StoreError::from_error)?,
            snapshot,
        ),
        Inspection::Control { action, snapshot } => (
            json!({ "type": match action {
                ControlAction::Reset => "reset",
                ControlAction::Undo => "undo",
                ControlAction::Redo => "redo",
            } }),
            snapshot,
        ),
    };
    let snapshot = identities.borrow_mut().snapshot(snapshot);
    invoke(callback, &json!({ "event": event, "snapshot": snapshot }))
}
