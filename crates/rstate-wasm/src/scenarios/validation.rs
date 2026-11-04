use rstate::{StoreConfig, Update};
use serde_json::json;

use super::{Context, Event, Message, Recorder, counter};

pub fn configuration(context: Context, record: Recorder) -> StoreConfig<Context, Event, Message> {
    StoreConfig::new(context, move |context, event, enqueue| {
        if matches!(event, Event::Start) {
            let effect_record = record.clone();
            enqueue.effect(move |scope| {
                effect_record(json!({
                    "kind": "effect",
                    "count": scope.get_snapshot().context.count
                }))
            });
            enqueue.emit(Message::Receipt {
                total: context.total,
            });
            enqueue.trigger(Event::Inc { by: 1 });
            Ok(Update::Keep)
        } else {
            counter::composition(context, event, enqueue)
        }
    })
}
