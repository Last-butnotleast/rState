mod cache;
mod lifecycle;
mod notifications;

use rstate::{Selection, Store};

use super::{Context, Event, store};

type Counter = Store<Context, Event>;

fn selected(store: &Counter) -> Selection<i32> {
    store.select(|context| context.count, |left, right| left == right)
}
