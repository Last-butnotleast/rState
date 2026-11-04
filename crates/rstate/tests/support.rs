use std::rc::Rc;

use rstate::{Store, StoreConfig, StoreError, Update};

#[derive(Debug, PartialEq)]
pub struct Context {
    pub count: i32,
    pub label: String,
}

pub enum Event {
    Add(i32),
    Keep,
    Same,
    Equal,
    Fail,
}

pub fn counter() -> Store<Context, Event> {
    Store::new(StoreConfig::new(
        Context {
            count: 0,
            label: "counter".into(),
        },
        |context: &Rc<Context>, event: &Event, _| match event {
            Event::Add(by) => Ok(Update::replace(Context {
                count: context.count + by,
                label: context.label.clone(),
            })),
            Event::Keep => Ok(Update::Keep),
            Event::Same => Ok(Update::Replace(Rc::clone(context))),
            Event::Equal => Ok(Update::replace(Context {
                count: context.count,
                label: context.label.clone(),
            })),
            Event::Fail => Err(StoreError::new("reducer failed")),
        },
    ))
}
