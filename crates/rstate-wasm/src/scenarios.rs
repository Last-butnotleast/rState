mod context;
mod counter;
mod factory;
mod ordering;

use std::rc::Rc;

use rstate::{Enqueue, Store, StoreConfig, StoreResult};
use serde_json::Value;

pub use context::{Context, Event, Message};
pub use factory::definition as factory;

pub type Recorder = Rc<dyn Fn(Value) -> StoreResult>;
pub type Fixture = Store<Context, Event, Message>;
pub type Queue = Enqueue<Context, Event, Message>;

pub fn create(name: &str, recorder: Recorder) -> Fixture {
    let name = name.to_owned();
    Store::new(StoreConfig::new(
        Context::default(),
        move |context, event, enqueue| match name.as_str() {
            "composition" => counter::composition(context, event, enqueue),
            "ordering" => ordering::reduce(context, event, enqueue, recorder.clone()),
            _ => counter::reduce(context, event),
        },
    ))
}
