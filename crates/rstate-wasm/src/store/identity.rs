use std::rc::Rc;

use rstate::Snapshot;
use serde_json::{Value, json};

use crate::scenarios::Context;

#[derive(Default)]
pub struct Identities {
    snapshots: Vec<Rc<Snapshot<Context>>>,
    contexts: Vec<Rc<Context>>,
}

impl Identities {
    pub fn snapshot(&mut self, snapshot: Rc<Snapshot<Context>>) -> Value {
        let snapshot_id = self
            .snapshots
            .iter()
            .position(|value| Rc::ptr_eq(value, &snapshot));
        let snapshot_id = match snapshot_id {
            Some(index) => index + 1,
            None => {
                self.snapshots.push(snapshot.clone());
                self.snapshots.len()
            }
        };
        let context_id = self
            .contexts
            .iter()
            .position(|value| Rc::ptr_eq(value, &snapshot.context));
        let context_id = match context_id {
            Some(index) => index + 1,
            None => {
                self.contexts.push(snapshot.context.clone());
                self.contexts.len()
            }
        };
        json!({"context": snapshot.context.as_ref(), "snapshotId": snapshot_id, "contextId": context_id})
    }
}
