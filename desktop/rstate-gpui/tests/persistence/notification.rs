use std::rc::Rc;

use gpui_kit::{AppContext, TestAppContext};
use rstate::{PersistenceActivity, PersistenceOutcome};
use rstate_gpui::ViewBindings;

use crate::support::{Event, View, observe};

use super::support::{Storage, store};

#[gpui_kit::test]
fn status_and_selection_bindings_share_a_target_and_read_live_recovery(cx: &mut TestAppContext) {
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
    let binding = binding.unwrap();
    let notifications = observe(cx, &view);
    assert_eq!(binding.get().sequence(), 0);
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
    storage.fail.set(true);
    store.send(Event::Set(1)).unwrap();
    assert!(binding.get().error().is_some());
    storage.fail.set(false);
    store.save().unwrap();
    assert!(binding.get().error().is_none());
    assert_eq!(
        binding.get().last_activity(),
        Some(PersistenceActivity::Save)
    );
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    assert_eq!(view.read_with(cx, |view, _| *view.values[0].get()), 1);
}

#[gpui_kit::test]
fn explicit_status_operations_notify_without_changing_domain_snapshot(cx: &mut TestAppContext) {
    let storage = Storage::default();
    let store = store(&storage).unwrap();
    let mut binding = None;
    let view = cx.new(|cx| {
        let target = ViewBindings::<View>::new(cx);
        binding = Some(target.bind_persistence(&store).unwrap());
        View {
            target,
            values: Vec::new(),
        }
    });
    let binding = binding.unwrap();
    let notifications = observe(cx, &view);
    let snapshot = store.get_snapshot();
    store.save().unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    store.clear_storage().unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
    assert_eq!(
        binding.get().last_activity(),
        Some(PersistenceActivity::Clear)
    );
    assert!(matches!(
        binding.get().outcome(),
        Some(PersistenceOutcome::Succeeded)
    ));
    assert!(Rc::ptr_eq(&snapshot, &store.get_snapshot()));
}
