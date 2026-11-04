use rstate::{
    PathSegment, Schema, SchemaCheck, SchemaMetadata, Schemas, StoreError, StoreResult,
    ValidationIssue,
};
use serde_json::{Value, json};

use super::configuration::Configuration;
use crate::scenarios::{Context, Event, Message, Recorder};

pub fn definitions(
    configuration: &Configuration,
    record: Recorder,
) -> Schemas<Context, Event, Message> {
    if configuration.no_schemas {
        return Schemas::default();
    }
    let context_record = record.clone();
    let event_record = record.clone();
    let missing_receipt = configuration.missing_receipt;
    Schemas::default()
        .with_context(Schema::new(
            metadata("context"),
            move |context: &Context| {
                context_record(
                    json!({ "kind": "validate", "target": "context", "value": context }),
                )?;
                context_check(context)
            },
        ))
        .with_events(Schema::new(metadata("events"), move |event| {
            event_check(event, &event_record)
        }))
        .with_emitted(Schema::new(metadata("emitted"), move |message| {
            emitted_check(message, missing_receipt, &record)
        }))
}

fn metadata(name: &str) -> SchemaMetadata {
    SchemaMetadata::new(name).with_vendor("rstate-fixture")
}

fn context_check(context: &Context) -> StoreResult<SchemaCheck> {
    if !(0..=10).contains(&context.count) {
        return Ok(invalid(
            "count must be between 0 and 10",
            vec![field("count")],
        ));
    }
    if let Some(index) = context.items.iter().position(|value| *value < 0) {
        return Ok(invalid(
            "price must be nonnegative",
            vec![field("items"), PathSegment::Index(index)],
        ));
    }
    let total = context.items.iter().try_fold(0_i64, |total, value| {
        total
            .checked_add(*value)
            .ok_or_else(|| StoreError::new("total overflow"))
    })?;
    if total != context.total {
        return Ok(invalid("total must equal item sum", vec![field("total")]));
    }
    Ok(SchemaCheck::Valid)
}

fn event_check(event: &Event, record: &Recorder) -> StoreResult<SchemaCheck> {
    let (name, field_name, value) = match event {
        Event::Inc { by } => ("inc", "by", *by),
        Event::Bump { by } => ("bump", "by", *by),
        Event::Add { price } => ("add", "price", *price),
        Event::Recalculate | Event::Receipt => {
            let name = if matches!(event, Event::Receipt) {
                "receipt"
            } else {
                "recalculate"
            };
            record(json!({ "kind": "validate", "target": "event", "name": name, "value": {} }))?;
            return Ok(invalid("event is internal only", Vec::new()).named(name));
        }
        Event::Unknown => return Ok(SchemaCheck::unknown("unknown")),
        Event::Keep
        | Event::Same
        | Event::Equal
        | Event::Fail
        | Event::Start
        | Event::EffectFail => {
            let name = match event {
                Event::Keep => "keep",
                Event::Same => "same",
                Event::Equal => "equal",
                Event::Fail => "fail",
                Event::Start => "start",
                _ => "effectFail",
            };
            record(json!({ "kind": "validate", "target": "event", "name": name, "value": {} }))?;
            return Ok(SchemaCheck::Valid);
        }
    };
    let mut payload = serde_json::Map::new();
    payload.insert(field_name.to_owned(), json!(value));
    record(json!({
        "kind": "validate",
        "target": "event",
        "name": name,
        "value": Value::Object(payload)
    }))?;
    Ok(if (0..=5).contains(&value) {
        SchemaCheck::Valid
    } else {
        invalid(
            &format!("{field_name} must be between 0 and 5"),
            vec![field(field_name)],
        )
        .named(name)
    })
}

fn emitted_check(message: &Message, missing: bool, record: &Recorder) -> StoreResult<SchemaCheck> {
    if missing {
        return Ok(SchemaCheck::unknown("receipt"));
    }
    let Message::Receipt { total } = message;
    record(json!({
        "kind": "validate",
        "target": "emitted",
        "name": "receipt",
        "value": { "total": total }
    }))?;
    Ok(if *total > 3 {
        invalid("receipt total must not exceed 3", vec![field("total")]).named("receipt")
    } else {
        SchemaCheck::Valid
    })
}

fn invalid(message: &str, path: Vec<PathSegment>) -> SchemaCheck {
    SchemaCheck::invalid(vec![ValidationIssue::new(message).with_path(path)])
}

fn field(name: &str) -> PathSegment {
    PathSegment::Field(name.to_owned())
}
