use js_sys::{Error, JSON, Reflect};
use rstate::{PathSegment, ValidationError, ValidationReason, ValidationStage};
use serde_json::json;
use wasm_bindgen::JsValue;

pub fn annotate(error: &Error, validation: &ValidationError) {
    let reason = match validation.reason {
        ValidationReason::InvalidContext => "invalidContext",
        ValidationReason::InvalidEvent => "invalidEvent",
        ValidationReason::InvalidEmitted => "invalidEmitted",
        ValidationReason::UnknownEvent => "unknownEvent",
        ValidationReason::UnknownEmitted => "unknownEmitted",
        _ => "unknown",
    };
    let stage = match validation.stage {
        ValidationStage::InitialContext => "initialContext",
        ValidationStage::Event => "event",
        ValidationStage::Context => "context",
        ValidationStage::Emitted => "emitted",
        _ => "unknown",
    };
    let issues: Vec<_> = validation
        .issues
        .iter()
        .map(|issue| {
            let path: Vec<_> = issue
                .path
                .iter()
                .map(|segment| match segment {
                    PathSegment::Field(value) => json!(value),
                    PathSegment::Index(value) => json!(value),
                })
                .collect();
            json!({ "message": issue.message, "path": path })
        })
        .collect();
    let _ = Reflect::set(
        error,
        &JsValue::from_str("reason"),
        &JsValue::from_str(reason),
    );
    let _ = Reflect::set(
        error,
        &JsValue::from_str("stage"),
        &JsValue::from_str(stage),
    );
    if let Some(name) = &validation.name {
        let _ = Reflect::set(
            error,
            &JsValue::from_str("eventType"),
            &JsValue::from_str(name),
        );
    }
    if let Ok(value) = JSON::parse(&json!(issues).to_string()) {
        let _ = Reflect::set(error, &JsValue::from_str("issues"), &value);
    }
    if let Ok(value) = JSON::parse(
        &json!({
            "name": validation.schema.name,
            "vendor": validation.schema.vendor
        })
        .to_string(),
    ) {
        let _ = Reflect::set(error, &JsValue::from_str("schema"), &value);
    }
}
