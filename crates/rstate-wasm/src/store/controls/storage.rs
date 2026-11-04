use std::error::Error;

use js_sys::Function;
use rstate::{SnapshotStorage, StoreError, StoreResult};
use wasm_bindgen::JsValue;

pub(in crate::store) struct CallbackStorage {
    pub read: Function,
    pub write: Function,
    pub remove: Function,
}

impl SnapshotStorage for CallbackStorage {
    fn read(&self, name: &str) -> StoreResult<Option<Vec<u8>>> {
        let value = self
            .read
            .call1(&JsValue::UNDEFINED, &JsValue::from_str(name))
            .map_err(|failure| callback_error(&failure))?;
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }
        value
            .as_string()
            .map(|value| Some(value.into_bytes()))
            .ok_or_else(|| StoreError::new("storage returned a non-string value"))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> StoreResult {
        let text = std::str::from_utf8(bytes).map_err(StoreError::from_error)?;
        self.write
            .call2(
                &JsValue::UNDEFINED,
                &JsValue::from_str(name),
                &JsValue::from_str(text),
            )
            .map(|_| ())
            .map_err(|failure| callback_error(&failure))
    }

    fn remove(&self, name: &str) -> StoreResult {
        self.remove
            .call1(&JsValue::UNDEFINED, &JsValue::from_str(name))
            .map(|_| ())
            .map_err(|failure| callback_error(&failure))
    }
}

fn callback_error(value: &JsValue) -> StoreError {
    let message = js_sys::Reflect::get(value, &JsValue::from_str("message"))
        .ok()
        .and_then(|value| value.as_string())
        .or_else(|| value.as_string())
        .unwrap_or_else(|| "storage callback failed".to_owned());
    StoreError::new(message)
}

pub(super) fn original_message(error: &(dyn Error + 'static)) -> String {
    let mut current = error;
    while let Some(source) = current.source() {
        current = source;
    }
    current.to_string()
}
