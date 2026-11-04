use std::rc::Rc;

use js_sys::Function;
use rstate::{JsonCodec, Persistence, Store};
use serde::Deserialize;
use serde_json::json;
use wasm_bindgen::prelude::*;

use super::storage::{CallbackStorage, original_message};
use crate::scenarios::{self, Context, Fixture};
use crate::store::handle::{FixtureStore, error, invoke};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Configuration {
    #[serde(default = "counter")]
    scenario: String,
    #[serde(default)]
    initial_count: i64,
    history_limit: Option<usize>,
    persistence: Option<PersistenceConfiguration>,
}

#[derive(Deserialize)]
struct PersistenceConfiguration {
    name: String,
    version: u64,
}

#[wasm_bindgen]
pub struct FixtureControls {
    store: Fixture,
}

#[wasm_bindgen]
impl FixtureControls {
    #[wasm_bindgen(constructor)]
    pub fn new(
        serialized: &str,
        recorder: Function,
        read: Option<Function>,
        write: Option<Function>,
        remove: Option<Function>,
        on_error: Option<Function>,
    ) -> Result<Self, JsValue> {
        let configuration: Configuration = serde_json::from_str(serialized)
            .map_err(|failure| JsValue::from_str(&failure.to_string()))?;
        let record = Rc::new(move |entry| invoke(&recorder, &entry));
        let mut config = scenarios::configuration(
            &configuration.scenario,
            Context {
                count: configuration.initial_count,
                ..Context::default()
            },
            record,
        );
        if let Some(limit) = configuration.history_limit {
            config = config.with_history(limit);
        }
        let store = if let Some(options) = configuration.persistence {
            let storage = CallbackStorage {
                read: read.ok_or_else(|| JsValue::from_str("missing storage reader"))?,
                write: write.ok_or_else(|| JsValue::from_str("missing storage writer"))?,
                remove: remove.ok_or_else(|| JsValue::from_str("missing storage remover"))?,
            };
            let persistence = Persistence::new(
                options.name,
                options.version,
                storage,
                JsonCodec::<Context>::new(),
            )
            .on_error(move |failure| {
                if let Some(callback) = &on_error {
                    let _ = invoke(callback, &json!(original_message(failure)));
                }
            });
            Store::new_persisted(config, persistence)
                .map_err(|failure| js_sys::Error::new(&failure.to_string()))?
        } else {
            Store::new(config)
        };
        Ok(Self { store })
    }

    pub fn base(&self) -> FixtureStore {
        FixtureStore::from_store(self.store.clone())
    }

    pub fn reset(&self) -> Result<(), JsValue> {
        self.store.reset().map_err(|failure| error(&failure))
    }

    pub fn reset_with_count(&self, count: i64) -> Result<(), JsValue> {
        self.store
            .reset_with(move |initial, _current| {
                Ok(Rc::new(Context {
                    count,
                    ..initial.as_ref().clone()
                }))
            })
            .map_err(|failure| error(&failure))
    }

    pub fn undo(&self) -> Result<(), JsValue> {
        self.store.undo().map_err(|failure| error(&failure))
    }

    pub fn redo(&self) -> Result<(), JsValue> {
        self.store.redo().map_err(|failure| error(&failure))
    }

    pub fn history(&self) -> String {
        let history = self.store.history_status();
        json!({ "past": history.past, "future": history.future, "limit": history.limit })
            .to_string()
    }

    pub fn save(&self) -> Result<(), JsValue> {
        self.store
            .save()
            .map_err(|failure| js_sys::Error::new(&failure.to_string()).into())
    }

    pub fn clear_storage(&self) -> Result<(), JsValue> {
        self.store
            .clear_storage()
            .map_err(|failure| js_sys::Error::new(&failure.to_string()).into())
    }

    pub fn persistence_error(&self) -> Option<String> {
        self.store
            .persistence_error()
            .map(|failure| original_message(failure.as_ref()))
    }

    pub fn persistence_status(&self) -> String {
        super::status::read(&self.store)
    }
}

fn counter() -> String {
    "counter".to_owned()
}
