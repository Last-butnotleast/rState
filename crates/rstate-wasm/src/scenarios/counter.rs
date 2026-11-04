use std::rc::Rc;

use rstate::{StoreError, StoreResult, Update};

use super::{Context, Event, Message, Queue};

pub fn reduce(context: &Rc<Context>, event: &Event) -> StoreResult<Update<Context>> {
    match event {
        Event::Inc { by } | Event::Bump { by } => Ok(Update::replace(Context {
            count: context.count + by,
            ..context.as_ref().clone()
        })),
        Event::Same => Ok(Update::Replace(context.clone())),
        Event::Equal => Ok(Update::replace(context.as_ref().clone())),
        Event::Fail => Err(StoreError::new("reducer failed")),
        _ => Ok(Update::Keep),
    }
}

pub fn composition(
    context: &Rc<Context>,
    event: &Event,
    enqueue: &mut Queue,
) -> StoreResult<Update<Context>> {
    match event {
        Event::Add { price } => {
            let mut next = context.as_ref().clone();
            next.items.push(*price);
            enqueue.trigger(Event::Recalculate);
            Ok(Update::replace(next))
        }
        Event::Recalculate => {
            enqueue.trigger(Event::Receipt);
            Ok(Update::replace(Context {
                total: context.items.iter().sum(),
                ..context.as_ref().clone()
            }))
        }
        Event::Receipt => {
            enqueue.emit(Message::Receipt {
                total: context.total,
            });
            Ok(Update::Keep)
        }
        _ => reduce(context, event),
    }
}
