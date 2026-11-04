use std::{cell::Cell, rc::Rc};

use gpui_kit::{AppContext, TestAppContext};
use rstate::{Store, StoreConfig, Update};
use rstate_gpui::ViewBindings;

use crate::support::{Event, counter, observe, view};

#[gpui_kit::test]
fn dropped_binding_invalidates_queued_work(cx: &mut TestAppContext) {
    let store = counter();
    let view = view(cx, &store, 1);
    let notifications = observe(cx, &view);
    store.send(Event::Set(1)).unwrap();
    view.update(cx, |view, _| view.values.clear());
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
    store.send(Event::Set(2)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
    view.update(cx, |view, _| {
        view.values.push(
            view.target
                .bind(store.select(|value| *value, PartialEq::eq)),
        );
    });
    store.send(Event::Set(3)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
}

#[gpui_kit::test]
fn replacement_keeps_other_live_dirty_work(cx: &mut TestAppContext) {
    let store = counter();
    let view = view(cx, &store, 1);
    let notifications = observe(cx, &view);
    store.send(Event::Set(1)).unwrap();
    view.update(cx, |view, _| {
        view.values.clear();
        view.values.push(
            view.target
                .bind(store.select(|value| *value, PartialEq::eq)),
        );
    });
    store.send(Event::Set(2)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    assert_eq!(view.read_with(cx, |view, _| *view.values[0].get()), 2);
    store.send(Event::Set(3)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
}

#[gpui_kit::test]
fn released_entity_does_not_skip_effects_and_binding_owns_source(cx: &mut TestAppContext) {
    let effects = Rc::new(Cell::new(0_usize));
    let effect_count = effects.clone();
    let store = Store::new(StoreConfig::<i32, i32>::new(0, move |_, event, enqueue| {
        let count = effect_count.clone();
        enqueue.effect(move |_| {
            count.set(count.get().saturating_add(1));
            Ok(())
        });
        Ok(Update::replace(*event))
    }));
    let weak = store.downgrade();
    let mut binding = None;
    let entity = cx.new(|cx| {
        let target = ViewBindings::<()>::new(cx);
        binding = Some(target.bind(store.select(|value| *value, PartialEq::eq)));
    });
    let binding = binding.unwrap();
    store.send(1).unwrap();
    drop(entity);
    cx.run_until_parked();
    store.send(2).unwrap();
    cx.run_until_parked();
    assert_eq!(effects.get(), 2);
    assert_eq!(*binding.get(), 2);
    drop(store);
    assert!(weak.upgrade().is_some());
    drop(binding);
    assert!(weak.upgrade().is_none());
}

#[gpui_kit::test]
fn closing_one_view_leaves_other_view_active(cx: &mut TestAppContext) {
    let store = counter();
    let first = view(cx, &store, 1);
    let second = view(cx, &store, 1);
    let first_count = observe(cx, &first);
    let second_count = observe(cx, &second);
    store.send(Event::Set(1)).unwrap();
    drop(first);
    cx.run_until_parked();
    assert_eq!(first_count.get(), 0);
    assert_eq!(second_count.get(), 1);
    store.send(Event::Set(2)).unwrap();
    cx.run_until_parked();
    assert_eq!(second_count.get(), 2);
    assert_eq!(second.read_with(cx, |view, _| *view.values[0].get()), 2);
}

#[gpui_kit::test]
fn detached_notification_does_not_keep_source_store_alive(cx: &mut TestAppContext) {
    let store = counter();
    let weak = store.downgrade();
    let view = view(cx, &store, 1);
    let notifications = observe(cx, &view);
    store.send(Event::Set(1)).unwrap();
    view.update(cx, |view, _| view.values.clear());
    drop(store);
    assert!(weak.upgrade().is_none());
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
}
