use std::{cell::RefCell, rc::Rc};

use gpui_kit::TestAppContext;

use crate::support::{Event, counter, observe, view};

#[gpui_kit::test]
fn notification_can_dispatch_and_schedule_followup_work(cx: &mut TestAppContext) {
    let store = counter();
    let view = view(cx, &store, 1);
    let notifications = observe(cx, &view);
    let weak = store.downgrade();
    cx.update(|cx| {
        cx.observe(&view, move |_, _| {
            let store = weak.upgrade().unwrap();
            if *store.get_snapshot().context == 1 {
                store.send(Event::Set(2)).unwrap();
            }
        })
        .detach();
    });
    store.send(Event::Set(1)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
    assert_eq!(view.read_with(cx, |view, _| *view.values[0].get()), 2);
}

#[gpui_kit::test]
fn nested_same_and_cross_store_sends_remain_synchronous_in_gpui_handler(cx: &mut TestAppContext) {
    let first = counter();
    let second = counter();
    let first_view = view(cx, &first, 1);
    let second_view = view(cx, &second, 1);
    let first_count = observe(cx, &first_view);
    let second_count = observe(cx, &second_view);
    let order = Rc::new(RefCell::new(Vec::new()));
    let weak = first.downgrade();
    let other = second.downgrade();
    let trace = order.clone();
    let _guard = first
        .subscribe(move |snapshot| {
            trace.borrow_mut().push(*snapshot.context);
            if *snapshot.context == 1 {
                let store = weak.upgrade().unwrap();
                store.send(Event::Set(2))?;
                trace.borrow_mut().push(*store.get_snapshot().context);
                other.upgrade().unwrap().send(Event::Set(9))?;
                trace.borrow_mut().push(3);
            }
            Ok(())
        })
        .scoped();
    first_view.update(cx, |view, _| {
        first.send(Event::Set(1)).unwrap();
        assert_eq!(*view.values[0].get(), 2);
        assert_eq!(*second.get_snapshot().context, 9);
        assert_eq!(&*order.borrow(), &[1, 2, 3]);
    });
    cx.run_until_parked();
    assert_eq!((first_count.get(), second_count.get()), (1, 1));
    assert_eq!(
        second_view.read_with(cx, |view, _| *view.values[0].get()),
        9
    );
}
