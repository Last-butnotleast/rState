use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use js_sys::Function;
use rstate::{StoreError, StoreResult, Subscription};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use super::{identity::Identities, inspection};
use crate::scenarios::{self, Event, Fixture, Message};

#[wasm_bindgen]
pub struct FixtureStore {
    store: Fixture,
    identities: Rc<RefCell<Identities>>,
    subscriptions: RefCell<HashMap<u32, Subscription>>,
    next_subscription: Cell<u32>,
}

#[wasm_bindgen]
impl FixtureStore {
    #[wasm_bindgen(constructor)]
    pub fn new(scenario: &str, recorder: Function) -> Result<Self, JsValue> {
        if !matches!(scenario, "counter" | "composition" | "ordering") {
            return Err(JsValue::from_str("unknown scenario"));
        }
        let record = Rc::new(move |entry: Value| invoke(&recorder, &entry));
        Ok(Self::from_store(scenarios::create(scenario, record)))
    }

    pub fn send(&self, event: &str) -> Result<(), JsValue> {
        self.store
            .send(parse(event)?)
            .map_err(|value| error(&value))
    }

    pub fn can(&self, event: &str) -> Result<bool, JsValue> {
        self.store.can(parse(event)?).map_err(|value| error(&value))
    }

    pub fn snapshot(&self) -> String {
        self.identities
            .borrow_mut()
            .snapshot(&self.store.get_snapshot())
            .to_string()
    }

    pub fn initial_snapshot(&self) -> String {
        self.identities
            .borrow_mut()
            .snapshot(&self.store.get_initial_snapshot())
            .to_string()
    }

    pub fn transition(&self, event: &str) -> Result<String, JsValue> {
        let result = self
            .store
            .transition(self.store.get_snapshot(), parse(event)?)
            .map_err(|value| error(&value))?;
        let snapshot = self.identities.borrow_mut().snapshot(&result.snapshot);
        Ok(json!({"snapshot": snapshot, "effects": result.effects.len(), "allowed": result.allowed}).to_string())
    }

    pub fn subscribe(&self, callback: Function) -> Result<u32, JsValue> {
        let id = self.next_id()?;
        let identities = self.identities.clone();
        let subscription = self.store.subscribe(move |snapshot| {
            let value = identities.borrow_mut().snapshot(&snapshot);
            invoke(&callback, &value)
        });
        Ok(self.register(id, subscription))
    }

    pub fn inspect(&self, callback: Function) -> Result<u32, JsValue> {
        let id = self.next_id()?;
        let identities = self.identities.clone();
        let subscription = self
            .store
            .inspect(move |record| inspection::notify(&record, &identities, &callback))
            .map_err(|value| error(&value))?;
        Ok(self.register(id, subscription))
    }

    pub fn on_any(&self, callback: Function) -> Result<u32, JsValue> {
        let id = self.next_id()?;
        Ok(self.register(
            id,
            self.store.on_any(move |message| {
                invoke(
                    &callback,
                    &serde_json::to_value(message.as_ref()).map_err(StoreError::from_error)?,
                )
            }),
        ))
    }

    pub fn on_receipt(&self, callback: Function) -> Result<u32, JsValue> {
        let id = self.next_id()?;
        Ok(self.register(
            id,
            self.store.on(
                |message| matches!(message, Message::Receipt { .. }),
                move |message| {
                    invoke(
                        &callback,
                        &serde_json::to_value(message.as_ref()).map_err(StoreError::from_error)?,
                    )
                },
            ),
        ))
    }

    pub fn subscribe_selected(&self, field: &str, callback: Function) -> Result<u32, JsValue> {
        let total = match field {
            "count" => false,
            "total" => true,
            _ => return Err(JsValue::from_str("unknown selection")),
        };
        let id = self.next_id()?;
        let selection = self.store.select(
            move |context| if total { context.total } else { context.count },
            |left, right| left == right,
        );
        Ok(self.register(
            id,
            selection.subscribe(move |value| invoke(&callback, &json!(*value))),
        ))
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

impl FixtureStore {
    pub(super) fn from_store(store: Fixture) -> Self {
        Self {
            store,
            identities: Rc::new(RefCell::new(Identities::default())),
            subscriptions: RefCell::new(HashMap::new()),
            next_subscription: Cell::new(0),
        }
    }

    fn next_id(&self) -> Result<u32, JsValue> {
        let id = self
            .next_subscription
            .get()
            .checked_add(1)
            .ok_or_else(|| JsValue::from_str("subscription limit reached"))?;
        self.next_subscription.set(id);
        Ok(id)
    }

    fn register(&self, id: u32, subscription: Subscription) -> u32 {
        self.subscriptions.borrow_mut().insert(id, subscription);
        id
    }
}

fn parse(event: &str) -> Result<Event, JsValue> {
    serde_json::from_str(event).map_err(|value| JsValue::from_str(&value.to_string()))
}

pub(super) fn invoke(callback: &Function, value: &Value) -> StoreResult {
    callback
        .call1(&JsValue::UNDEFINED, &JsValue::from_str(&value.to_string()))
        .map(|_| ())
        .map_err(|value| {
            let message = js_sys::Reflect::get(&value, &JsValue::from_str("message"))
                .ok()
                .and_then(|value| value.as_string())
                .or_else(|| value.as_string())
                .unwrap_or_else(|| "callback failed".to_owned());
            StoreError::new(message)
        })
}

pub(super) fn error(value: &StoreError) -> JsValue {
    let error = js_sys::Error::new(&value.to_string());
    if let Some(validation) = value.validation_error() {
        super::validation::error::annotate(&error, validation);
    }
    error.into()
}
