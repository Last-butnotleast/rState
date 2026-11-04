use std::rc::Rc;

use js_sys::Function;
use rstate::{JsonCodec, Persistence, Store};
use serde_json::json;
use wasm_bindgen::prelude::*;

use super::configuration::Configuration;
use super::schemas;
use crate::scenarios::{self, Context, Fixture};
use crate::store::controls::storage::CallbackStorage;
use crate::store::handle::{FixtureStore, error, invoke};

#[wasm_bindgen]
pub struct FixtureValidation {
    store: Fixture,
}

#[wasm_bindgen]
impl FixtureValidation {
    #[wasm_bindgen(constructor)]
    pub fn new(
        serialized: &str,
        recorder: Function,
        read: Option<Function>,
        write: Option<Function>,
        remove: Option<Function>,
    ) -> Result<Self, JsValue> {
        let configuration: Configuration = serde_json::from_str(serialized)
            .map_err(|failure| JsValue::from_str(&failure.to_string()))?;
        let record = Rc::new(move |entry| invoke(&recorder, &entry));
        let schemas = schemas::definitions(&configuration, record.clone());
        let mut config = scenarios::validation_configuration(
            Context {
                count: configuration.initial_count,
                ..Context::default()
            },
            record,
        )
        .with_schemas(schemas);
        if let Some(limit) = configuration.history_limit {
            config = config.with_history(limit);
        }
        let options = configuration.options();
        let store = match (configuration.runtime, configuration.persistence) {
            (runtime, Some(persistence)) => {
                let storage = CallbackStorage {
                    read: read.ok_or_else(|| JsValue::from_str("missing storage reader"))?,
                    write: write.ok_or_else(|| JsValue::from_str("missing storage writer"))?,
                    remove: remove.ok_or_else(|| JsValue::from_str("missing storage remover"))?,
                };
                let persistence = Persistence::new(
                    persistence.name,
                    persistence.version,
                    storage,
                    JsonCodec::<Context>::new(),
                );
                if runtime {
                    Store::new_persisted_validated(config, persistence, options)
                } else {
                    Store::new_persisted(config, persistence)
                }
                .map_err(|failure| error(&failure))?
            }
            (true, None) => {
                Store::new_validated(config, options).map_err(|failure| error(&failure))?
            }
            (false, None) => Store::new(config),
        };
        Ok(Self { store })
    }

    pub fn base(&self) -> FixtureStore {
        FixtureStore::from_store(self.store.clone())
    }

    pub fn metadata(&self) -> String {
        let schemas = self.store.schemas();
        json!({
            "context": schemas.context().is_some(),
            "events": schemas.events().is_some(),
            "emitted": schemas.emitted().is_some()
        })
        .to_string()
    }

    pub fn history(&self) -> String {
        let history = self.store.history_status();
        json!({ "past": history.past, "future": history.future, "limit": history.limit })
            .to_string()
    }

    pub fn reset(&self) -> Result<(), JsValue> {
        self.store.reset().map_err(|failure| error(&failure))
    }

    pub fn reset_with_count(&self, count: i64) -> Result<(), JsValue> {
        self.store
            .reset_with(move |initial, _| {
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
}
