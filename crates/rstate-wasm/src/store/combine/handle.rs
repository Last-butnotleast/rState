use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use js_sys::Function;
use rstate::{StoreError, Subscription};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use super::super::handle::{error, invoke};
use super::super::selection::value::{Identities, LabelValue, NumberValue};
use super::graph::Graph;
use crate::scenarios::Event;

#[wasm_bindgen]
pub struct FixtureCombined {
    graph: Graph,
    left_ids: Rc<RefCell<Identities<NumberValue>>>,
    right_ids: Rc<RefCell<Identities<NumberValue>>>,
    combined_ids: Rc<RefCell<Identities<NumberValue>>>,
    chain_ids: Rc<RefCell<Identities<LabelValue>>>,
    subscriptions: RefCell<HashMap<u32, Subscription>>,
    next_subscription: Cell<u32>,
}

#[wasm_bindgen]
impl FixtureCombined {
    #[wasm_bindgen(constructor)]
    pub fn new(mode: &str) -> Result<Self, JsValue> {
        Ok(Self {
            graph: Graph::new(mode)?,
            left_ids: Rc::new(RefCell::new(Identities::default())),
            right_ids: Rc::new(RefCell::new(Identities::default())),
            combined_ids: Rc::new(RefCell::new(Identities::default())),
            chain_ids: Rc::new(RefCell::new(Identities::default())),
            subscriptions: RefCell::new(HashMap::new()),
            next_subscription: Cell::new(0),
        })
    }

    pub fn read(&self, target: &str) -> Result<String, JsValue> {
        let value = match target {
            "left" => number(&self.graph.left.get(), &self.left_ids),
            "right" => number(&self.graph.right.get(), &self.right_ids),
            "combined" => number(&self.graph.combined.get(), &self.combined_ids),
            "chain" => label(&self.graph.chain.get(), &self.chain_ids),
            _ => return Err(JsValue::from_str("unknown combination target")),
        };
        Ok(value.to_string())
    }

    pub fn stats(&self) -> String {
        self.graph.counts.snapshot().to_string()
    }

    pub fn send(&self, input: &str, serialized: &str) -> Result<(), JsValue> {
        let store = match input {
            "left" => &self.graph.left_store,
            "right" => &self.graph.right_store,
            _ => return Err(JsValue::from_str("unknown combination input")),
        };
        let event: Event = serde_json::from_str(serialized)
            .map_err(|failure| error(&StoreError::from_error(failure)))?;
        store.send(event).map_err(|value| error(&value))
    }

    pub fn subscribe(&self, target: &str, callback: Function) -> Result<u32, JsValue> {
        let id = self
            .next_subscription
            .get()
            .checked_add(1)
            .ok_or_else(|| JsValue::from_str("subscription limit reached"))?;
        self.next_subscription.set(id);
        let subscription = match target {
            "left" => {
                let ids = self.left_ids.clone();
                self.graph
                    .left
                    .subscribe(move |value| invoke(&callback, &number(&value, &ids)))
            }
            "right" => {
                let ids = self.right_ids.clone();
                self.graph
                    .right
                    .subscribe(move |value| invoke(&callback, &number(&value, &ids)))
            }
            "combined" => {
                let ids = self.combined_ids.clone();
                self.graph
                    .combined
                    .subscribe(move |value| invoke(&callback, &number(&value, &ids)))
            }
            "chain" => {
                let ids = self.chain_ids.clone();
                self.graph
                    .chain
                    .subscribe(move |value| invoke(&callback, &label(&value, &ids)))
            }
            _ => return Err(JsValue::from_str("unknown combination target")),
        };
        self.subscriptions.borrow_mut().insert(id, subscription);
        Ok(id)
    }

    pub fn unsubscribe(&self, id: u32) {
        let subscription = self.subscriptions.borrow_mut().remove(&id);
        if let Some(subscription) = subscription {
            subscription.unsubscribe();
        }
    }

    pub fn dispose(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions.into_values() {
            subscription.unsubscribe();
        }
    }
}

fn number(value: &Rc<NumberValue>, ids: &RefCell<Identities<NumberValue>>) -> Value {
    let id = ids.borrow_mut().identify(value.clone());
    json!({ "value": value.value, "id": id })
}

fn label(value: &Rc<LabelValue>, ids: &RefCell<Identities<LabelValue>>) -> Value {
    let id = ids.borrow_mut().identify(value.clone());
    json!({ "value": value.value, "id": id })
}
