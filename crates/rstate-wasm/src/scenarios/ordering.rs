use std::rc::Rc;

use rstate::{StoreError, StoreResult, Update};
use serde_json::json;

use super::{Context, Event, Queue, Recorder, counter};

pub fn reduce(
    context: &Rc<Context>,
    event: &Event,
    enqueue: &mut Queue,
    recorder: Recorder,
) -> StoreResult<Update<Context>> {
    match event {
        Event::Start => {
            let first = recorder.clone();
            enqueue.effect(move |scope| {
                first(json!({"kind": "effect", "label": "before", "count": scope.get_snapshot().context.count}))?;
                scope.send(Event::Bump { by: 10 })
            });
            enqueue.effect(move |scope| {
                recorder(json!({"kind": "effect", "label": "after", "count": scope.get_snapshot().context.count}))
            });
            Ok(Update::replace(Context {
                count: context.count + 1,
                ..context.as_ref().clone()
            }))
        }
        Event::EffectFail => {
            enqueue.effect(|_| Err(StoreError::new("effect failed")));
            Ok(Update::replace(Context {
                count: context.count + 1,
                ..context.as_ref().clone()
            }))
        }
        _ => counter::reduce(context, event),
    }
}
