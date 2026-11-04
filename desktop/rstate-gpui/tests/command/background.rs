use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::TestAppContext;
use rstate::{Store, StoreConfig, Update};
use rstate_gpui::{CommandPolicy, CommandScope};

use super::support::{BackgroundGate, unexpected};

#[gpui::test]
fn background_result_returns_to_non_send_store_without_context_clone_bounds(
    cx: &mut TestAppContext,
) {
    struct Context(Rc<Cell<u32>>);
    let store = Store::new(StoreConfig::<Context, u32>::new(
        Context(Rc::new(Cell::new(0))),
        |_, value, _| Ok(Update::replace(Context(Rc::new(Cell::new(*value))))),
    ));
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let gate = BackgroundGate::default();
    let wait = gate.clone();
    let weak = store.downgrade();
    channel
        .submit_background(
            move || async move {
                wait.wait().await;
                9
            },
            move |value, _| weak.upgrade().unwrap().send(value),
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(store.get_snapshot().context.0.get(), 0);
    gate.release();
    cx.run_until_parked();
    assert_eq!(store.get_snapshot().context.0.get(), 9);
}

#[gpui::test]
fn expected_background_failure_is_typed_output_not_delivery_failure(cx: &mut TestAppContext) {
    let scope = cx.update(|cx| CommandScope::new(cx));
    let channel = scope.channel(CommandPolicy::Concurrent);
    let output = Rc::new(RefCell::new(None));
    let seen = output.clone();
    channel
        .submit_background(
            || async { Err::<u32, _>("service unavailable") },
            move |result, _| {
                seen.replace(Some(result));
                Ok(())
            },
            unexpected,
        )
        .unwrap();
    cx.run_until_parked();
    assert_eq!(*output.borrow(), Some(Err("service unavailable")));
}
