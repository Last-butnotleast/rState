use gpui::TestAppContext;

use crate::support::{Event, counter, observe, view};

#[gpui::test]
fn bindings_share_one_detached_job_and_later_changes_schedule_again(cx: &mut TestAppContext) {
    let store = counter();
    let view = view(cx, &store, 3);
    let notifications = observe(cx, &view);
    view.update(cx, |_, _| {
        store.send(Event::Set(1)).unwrap();
        store.send(Event::Set(2)).unwrap();
        assert_eq!(notifications.get(), 0);
    });
    assert_eq!(notifications.get(), 0);
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    assert_eq!(view.read_with(cx, |view, _| *view.values[0].get()), 2);
    store.send(Event::Set(3)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
}

#[gpui::test]
fn separate_view_targets_notify_independently(cx: &mut TestAppContext) {
    let store = counter();
    let first = view(cx, &store, 2);
    let second = view(cx, &store, 2);
    let first_count = observe(cx, &first);
    let second_count = observe(cx, &second);
    store.send(Event::Set(1)).unwrap();
    cx.run_until_parked();
    assert_eq!((first_count.get(), second_count.get()), (1, 1));
}
