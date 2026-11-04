use std::{cell::Cell, rc::Rc};

use gpui::{AppContext, Context, Entity, TestAppContext};
use rstate::{Store, StoreConfig, StoreError, Update};
use rstate_gpui::{SelectionBinding, ViewBindings};

pub enum Event {
    Set(i32),
    Keep,
    Fail,
}

pub fn counter() -> Store<i32, Event> {
    Store::new(
        StoreConfig::new(0, |_, event, _| match event {
            Event::Set(value) => Ok(Update::replace(*value)),
            Event::Keep => Ok(Update::Keep),
            Event::Fail => Err(StoreError::new("reducer failed")),
        })
        .with_history(4),
    )
}

pub struct View {
    pub target: ViewBindings<Self>,
    pub values: Vec<SelectionBinding<i32>>,
}

impl View {
    pub fn new(store: &Store<i32, Event>, count: usize, cx: &Context<Self>) -> Self {
        let target = ViewBindings::new(cx);
        let values = (0..count)
            .map(|_| target.bind(store.select(|value| *value, PartialEq::eq)))
            .collect();
        Self { target, values }
    }
}

pub fn view(cx: &mut TestAppContext, store: &Store<i32, Event>, count: usize) -> Entity<View> {
    cx.new(|cx| View::new(store, count, cx))
}

pub fn observe(cx: &TestAppContext, entity: &Entity<View>) -> Rc<Cell<usize>> {
    let notifications = Rc::new(Cell::new(0_usize));
    let count = notifications.clone();
    cx.update(|cx| {
        cx.observe(entity, move |_, _| {
            count.set(count.get().saturating_add(1));
        })
        .detach();
    });
    notifications
}
