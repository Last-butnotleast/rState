use rstate::StoreFactory;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::handle::FixtureStore;
use crate::scenarios::{self, Context, Event, Message};

#[derive(Deserialize)]
struct Input {
    count: i64,
}

#[wasm_bindgen]
pub struct FixtureFactory {
    factory: StoreFactory<i64, Context, Event, Message>,
}

#[wasm_bindgen]
impl FixtureFactory {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            factory: scenarios::factory(),
        }
    }

    pub fn create(&self, input: &str) -> Result<FixtureStore, JsValue> {
        let input: Input =
            serde_json::from_str(input).map_err(|error| js_sys::Error::new(&error.to_string()))?;
        self.factory
            .create(input.count)
            .map(FixtureStore::from_store)
            .map_err(|error| js_sys::Error::new(&error.to_string()).into())
    }
}

impl Default for FixtureFactory {
    fn default() -> Self {
        Self::new()
    }
}
