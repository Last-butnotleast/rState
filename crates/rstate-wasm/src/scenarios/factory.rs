use rstate::StoreFactory;

use super::{Context, Event, Message, counter};

pub fn definition() -> StoreFactory<i64, Context, Event, Message> {
    StoreFactory::new(
        |count| {
            Ok(Context {
                count,
                ..Context::default()
            })
        },
        counter::composition,
    )
}
