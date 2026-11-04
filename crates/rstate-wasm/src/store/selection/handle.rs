use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use js_sys::Function;
use rstate::{Selection, StoreError, Subscription};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use super::super::handle::{error, invoke};
use super::value::{Counts, Identities, LabelValue, NumberValue};
use crate::scenarios::{self, Event, Fixture};

#[wasm_bindgen]
pub struct FixtureSelection {
    store: Fixture,
    source: Selection<NumberValue>,
    mapped: Selection<NumberValue>,
    chain: Selection<LabelValue>,
    counts: Rc<Counts>,
    source_ids: Rc<RefCell<Identities<NumberValue>>>,
    mapped_ids: Rc<RefCell<Identities<NumberValue>>>,
    chain_ids: Rc<RefCell<Identities<LabelValue>>>,
    subscriptions: RefCell<HashMap<u32, Subscription>>,
    next_subscription: Cell<u32>,
}

#[wasm_bindgen]
impl FixtureSelection {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let store = scenarios::create("counter", Rc::new(|_| Ok(())));
        let counts = Rc::new(Counts::default());
        let source_counts = counts.clone();
        let source = store.select(
            move |context| {
                source_counts
                    .source
                    .set(source_counts.source.get().saturating_add(1));
                NumberValue {
                    value: context.count,
                }
            },
            |left, right| left.value == right.value,
        );
        let mapped_counts = counts.clone();
        let mapped = source.map(
            move |value| {
                mapped_counts
                    .mapped
                    .set(mapped_counts.mapped.get().saturating_add(1));
                NumberValue {
                    value: value.value.div_euclid(2),
                }
            },
            |left, right| left.value == right.value,
        );
        let chain_counts = counts.clone();
        let chain = mapped.map(
            move |value| {
                chain_counts
                    .chain
                    .set(chain_counts.chain.get().saturating_add(1));
                LabelValue {
                    value: format!("bucket:{}", value.value),
                }
            },
            |left, right| left.value == right.value,
        );
        Self {
            store,
            source,
            mapped,
            chain,
            counts,
            source_ids: Rc::new(RefCell::new(Identities::default())),
            mapped_ids: Rc::new(RefCell::new(Identities::default())),
            chain_ids: Rc::new(RefCell::new(Identities::default())),
            subscriptions: RefCell::new(HashMap::new()),
            next_subscription: Cell::new(0),
        }
    }

    pub fn read(&self, target: &str) -> Result<String, JsValue> {
        let value = match target {
            "source" => number(&self.source.get(), &self.source_ids),
            "mapped" => number(&self.mapped.get(), &self.mapped_ids),
            "chain" => label(&self.chain.get(), &self.chain_ids),
            _ => return Err(JsValue::from_str("unknown selection target")),
        };
        Ok(value.to_string())
    }

    pub fn stats(&self) -> String {
        self.counts.snapshot().to_string()
    }

    pub fn send(&self, serialized: &str) -> Result<(), JsValue> {
        let event: Event = serde_json::from_str(serialized)
            .map_err(|failure| error(&StoreError::from_error(failure)))?;
        self.store.send(event).map_err(|value| error(&value))
    }

    pub fn subscribe(&self, target: &str, callback: Function) -> Result<u32, JsValue> {
        let id = self
            .next_subscription
            .get()
            .checked_add(1)
            .ok_or_else(|| JsValue::from_str("subscription limit reached"))?;
        let subscription = match target {
            "source" => {
                let ids = self.source_ids.clone();
                self.source
                    .subscribe(move |value| invoke(&callback, &number(&value, &ids)))
            }
            "mapped" => {
                let ids = self.mapped_ids.clone();
                self.mapped
                    .subscribe(move |value| invoke(&callback, &number(&value, &ids)))
            }
            "chain" => {
                let ids = self.chain_ids.clone();
                self.chain
                    .subscribe(move |value| invoke(&callback, &label(&value, &ids)))
            }
            _ => return Err(JsValue::from_str("unknown selection target")),
        };
        self.next_subscription.set(id);
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

impl Default for FixtureSelection {
    fn default() -> Self {
        Self::new()
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
