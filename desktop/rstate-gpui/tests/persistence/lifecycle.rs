use gpui_kit::{AppContext, TestAppContext};
use rstate_gpui::ViewBindings;

use crate::support::{Event, View, counter, observe};

use super::support::{Storage, store};

#[gpui_kit::test]
fn dropped_status_binding_does_not_erase_another_dirty_selection(cx: &mut TestAppContext) {
    let storage = Storage::default();
    let store = store(&storage).unwrap();
    let mut binding = None;
    let view = cx.new(|cx| {
        let target = ViewBindings::<View>::new(cx);
        binding = Some(target.bind_persistence(&store).unwrap());
        View {
            values: vec![target.bind(store.select(|value| *value, PartialEq::eq))],
            target,
        }
    });
    let notifications = observe(cx, &view);
    store.send(Event::Set(1)).unwrap();
    drop(binding.take());
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    store.save().unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
}

#[gpui_kit::test]
fn status_binding_owns_store_but_released_entity_is_cleanup(cx: &mut TestAppContext) {
    let storage = Storage::default();
    let store = store(&storage).unwrap();
    let weak = store.downgrade();
    let mut binding = None;
    let entity = cx.new(|cx| {
        let bindings = ViewBindings::<()>::new(cx);
        binding = Some(bindings.bind_persistence(&store).unwrap());
    });
    store.save().unwrap();
    drop(entity);
    cx.run_until_parked();
    drop(store);
    let live = weak.upgrade().unwrap();
    live.save().unwrap();
    cx.run_until_parked();
    assert_eq!(binding.as_ref().unwrap().get().sequence(), 2);
    drop(live);
    drop(binding);
    assert!(weak.upgrade().is_none());
}

#[gpui_kit::test]
fn unconfigured_binding_fails_without_installing_a_registration(cx: &mut TestAppContext) {
    let store = counter();
    let view = cx.new(|cx| {
        let target = ViewBindings::<View>::new(cx);
        assert!(target.bind_persistence(&store).is_err());
        View {
            target,
            values: Vec::new(),
        }
    });
    let notifications = observe(cx, &view);
    store.send(Event::Set(1)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
}
