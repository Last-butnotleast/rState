use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{EffectScope, Store, StoreConfig, Update};

use crate::support::counter;

struct DropFlag(Rc<Cell<bool>>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

struct OpaqueContext {
    _resource: DropFlag,
}

struct OpaqueEvent;
struct OpaqueMessage;

#[test]
fn weak_clones_and_debug_add_no_bounds_and_upgrades_share_store_identity() {
    let dropped = Rc::new(Cell::new(false));
    let store: Store<OpaqueContext, OpaqueEvent, OpaqueMessage> = Store::new(StoreConfig::new(
        OpaqueContext {
            _resource: DropFlag(dropped.clone()),
        },
        |_, _, _| Ok(Update::Keep),
    ));
    let weak = store.downgrade();
    let alias = weak.clone();
    assert!(format!("{weak:?}").contains("alive: true"));
    let upgraded = alias.upgrade().unwrap();
    assert!(Rc::ptr_eq(&upgraded.get_snapshot(), &store.get_snapshot()));
    drop(upgraded);
    drop(store);
    assert!(dropped.get());
    assert!(weak.upgrade().is_none());
    assert!(alias.upgrade().is_none());
    assert!(format!("{weak:#?}").contains("alive: false"));
}

struct Count {
    value: i32,
    resource: Rc<DropFlag>,
}

#[test]
fn weak_callback_captures_allow_nested_sends_without_retaining_the_store() {
    let dropped = Rc::new(Cell::new(false));
    let store: Store<Count, i32> = Store::new(StoreConfig::new(
        Count {
            value: 0,
            resource: Rc::new(DropFlag(dropped.clone())),
        },
        |context: &Rc<Count>, by: &i32, _| {
            Ok(Update::replace(Count {
                value: context.value.checked_add(*by).unwrap(),
                resource: context.resource.clone(),
            }))
        },
    ));
    let weak = store.downgrade();
    let callback_store = weak.clone();
    drop(store.subscribe(move |snapshot| {
        if snapshot.context.value == 1 {
            if let Some(store) = callback_store.upgrade() {
                store.send(1)?;
            }
        }
        Ok(())
    }));
    store.send(1).unwrap();
    assert_eq!(store.get_snapshot().context.value, 2);
    drop(store);
    assert!(weak.upgrade().is_none());
    assert!(dropped.get());
}

#[test]
fn selections_are_intentional_store_owners_until_they_are_dropped() {
    let store = counter();
    let weak = store.downgrade();
    let selected = store.select(|context| context.count, |left, right| left == right);
    let mapped = selected.map(ToString::to_string, |left, right| left == right);
    drop(selected);
    drop(store);
    assert!(weak.upgrade().is_some());
    assert_eq!(&*mapped.get(), "0");
    drop(mapped);
    assert!(weak.upgrade().is_none());
}

#[test]
fn retained_effect_scopes_keep_their_store_alive() {
    let retained: Rc<RefCell<Option<EffectScope<i32, ()>>>> = Rc::new(RefCell::new(None));
    let recorder = Rc::downgrade(&retained);
    let store: Store<i32, ()> = Store::new(StoreConfig::new(0, move |_, (), enqueue| {
        let recorder = recorder.clone();
        enqueue.effect(move |scope| {
            if let Some(recorder) = recorder.upgrade() {
                recorder.replace(Some(scope));
            }
            Ok(())
        });
        Ok(Update::Keep)
    }));
    let weak = store.downgrade();
    store.send(()).unwrap();
    drop(store);
    assert!(weak.upgrade().is_some());
    let scope = retained.borrow_mut().take();
    drop(scope);
    assert!(weak.upgrade().is_none());
}
