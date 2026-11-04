use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{
    PathSegment, Schema, SchemaCheck, SchemaMetadata, Schemas, StoreConfig, StoreError, Update,
    ValidationIssue,
};

pub type Trace = Rc<RefCell<Vec<String>>>;

pub struct Context {
    pub count: i32,
    pub local: Rc<Cell<bool>>,
}

pub enum Event {
    Add(i32),
    Keep,
    Break,
    Repair,
    Start,
    Internal(i32),
    Emit(i32),
    UnknownEmission,
    Unknown,
    ProviderFailure,
    ReducerFailure,
}

pub enum Message {
    Value(i32),
    Unknown,
}

pub fn context(count: i32) -> Context {
    Context {
        count,
        local: Rc::new(Cell::new(false)),
    }
}

pub const fn label(event: &Event) -> &'static str {
    match event {
        Event::Add(_) => "add",
        Event::Keep => "keep",
        Event::Break => "break",
        Event::Repair => "repair",
        Event::Start => "start",
        Event::Internal(_) => "internal",
        Event::Emit(_) => "emit",
        Event::UnknownEmission => "unknownEmission",
        Event::Unknown => "unknown",
        Event::ProviderFailure => "providerFailure",
        Event::ReducerFailure => "reducerFailure",
    }
}

pub fn config(count: i32, trace: &Trace) -> StoreConfig<Context, Event, Message> {
    let trace = trace.clone();
    StoreConfig::new(context(count), move |current, event, enqueue| {
        trace.borrow_mut().push(format!("reduce:{}", label(event)));
        let next = match event {
            Event::Add(by) => {
                let next = current
                    .count
                    .checked_add(*by)
                    .ok_or_else(|| StoreError::new("counter overflow"))?;
                let trace = trace.clone();
                enqueue.effect(move |_| {
                    trace.borrow_mut().push("effect:add".into());
                    Ok(())
                });
                enqueue.emit(Message::Value(next));
                next
            }
            Event::Keep | Event::Unknown | Event::ProviderFailure => return Ok(Update::Keep),
            Event::Break => {
                let trace = trace.clone();
                enqueue.effect(move |_| {
                    trace.borrow_mut().push("effect:break".into());
                    Ok(())
                });
                enqueue.emit(Message::Value(1));
                -1
            }
            Event::Repair => {
                enqueue.trigger(Event::Internal(2));
                -1
            }
            Event::Start => {
                enqueue.trigger(Event::Internal(5));
                return Ok(Update::Keep);
            }
            Event::Internal(by) => current
                .count
                .checked_add(*by)
                .ok_or_else(|| StoreError::new("counter overflow"))?,
            Event::Emit(value) => {
                let before = trace.clone();
                enqueue.effect(move |_| {
                    before.borrow_mut().push("effect:before".into());
                    Ok(())
                });
                enqueue.emit(Message::Value(*value));
                let after = trace.clone();
                enqueue.effect(move |_| {
                    after.borrow_mut().push("effect:after".into());
                    Ok(())
                });
                return Ok(Update::Keep);
            }
            Event::UnknownEmission => {
                enqueue.emit(Message::Unknown);
                return Ok(Update::Keep);
            }
            Event::ReducerFailure => return Err(StoreError::new("reducer failed")),
        };
        Ok(Update::replace(Context {
            count: next,
            local: current.local.clone(),
        }))
    })
}

pub fn schemas(trace: &Trace, minimum: Rc<Cell<i32>>) -> Schemas<Context, Event, Message> {
    let context_trace = trace.clone();
    let event_trace = trace.clone();
    let emitted_trace = trace.clone();
    Schemas::default()
        .with_context(Schema::new(
            SchemaMetadata::new("counter").with_vendor("native-test"),
            move |value: &Context| {
                context_trace
                    .borrow_mut()
                    .push(format!("context:{}", value.count));
                Ok(if value.count < minimum.get() {
                    SchemaCheck::invalid(vec![
                        ValidationIssue::new("count below minimum")
                            .with_path(vec![PathSegment::Field("count".into())]),
                    ])
                } else {
                    SchemaCheck::Valid
                })
            },
        ))
        .with_events(Schema::new(
            SchemaMetadata::new("events"),
            move |event: &Event| {
                event_trace
                    .borrow_mut()
                    .push(format!("event:{}", label(event)));
                let check = match event {
                    Event::Add(by) if *by < 0 => SchemaCheck::invalid(vec![
                        ValidationIssue::new("by must be nonnegative")
                            .with_path(vec![PathSegment::Field("by".into())]),
                    ])
                    .named("add"),
                    Event::Internal(_) => SchemaCheck::invalid(vec![ValidationIssue::new(
                        "internal payload must not pass the outer validator",
                    )])
                    .named("internal"),
                    Event::Unknown => SchemaCheck::unknown("unknown"),
                    Event::ProviderFailure => {
                        return Err(StoreError::from_error(std::io::Error::other(
                            "provider failed",
                        )));
                    }
                    _ => SchemaCheck::Valid,
                };
                Ok(check)
            },
        ))
        .with_emitted(Schema::new(
            SchemaMetadata::new("messages"),
            move |message: &Message| {
                let check = match message {
                    Message::Value(value) => {
                        emitted_trace.borrow_mut().push(format!("emitted:{value}"));
                        if *value < 0 {
                            SchemaCheck::invalid(vec![
                                ValidationIssue::new("invalid value").with_path(vec![
                                    PathSegment::Field("rows".into()),
                                    PathSegment::Index(0),
                                    PathSegment::Field("value".into()),
                                ]),
                            ])
                            .named("value")
                        } else {
                            SchemaCheck::Valid
                        }
                    }
                    Message::Unknown => SchemaCheck::unknown("missing"),
                };
                Ok(check)
            },
        ))
}

pub fn trace() -> Trace {
    Rc::new(RefCell::new(Vec::new()))
}
