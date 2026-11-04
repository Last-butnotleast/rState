use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{Inspection, ScopedSubscription, Store, StoreConfig, Update};

use crate::support::{Event, counter};

#[test]
fn guards_cancel_on_drop_and_explicit_unsubscribe_is_shared_with_aliases() {
    let store = counter();
    let values = Rc::new(RefCell::new(Vec::new()));
    let seen = values.clone();
    let subscription = store.subscribe(move |snapshot| {
        seen.borrow_mut().push(snapshot.context.count);
        Ok(())
    });
    let alias = subscription.clone();
    let scoped = subscription.scoped();
    assert!(format!("{scoped:?}").contains("active: true"));
    store.send(Event::Add(1)).unwrap();
    scoped.unsubscribe();
    scoped.unsubscribe();
    alias.unsubscribe();
    assert!(format!("{scoped:#?}").contains("active: false"));
    drop(scoped);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*values.borrow(), [1]);
}

#[test]
fn a_scope_ends_its_registration_even_when_an_ordinary_alias_survives() {
    let store = counter();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    let subscription = store.subscribe(move |_| {
        observed.set(true);
        Ok(())
    });
    let alias = subscription.clone();
    {
        let _guard = subscription.scoped();
    }
    store.send(Event::Add(1)).unwrap();
    assert!(!called.get());
    assert!(format!("{alias:?}").contains("active: false"));
}

#[test]
fn mapped_guards_detach_without_eagerly_recomputing_on_later_updates() {
    let store = counter();
    let projections = Rc::new(Cell::new(0_u32));
    let count = projections.clone();
    let mapped = store
        .select(|context| context.count, |left, right| left == right)
        .map(
            move |value| {
                count.set(count.get().checked_add(1).unwrap());
                value.to_string()
            },
            |left, right| left == right,
        );
    let scoped = mapped.subscribe(|_| Ok(())).scoped();
    assert_eq!(projections.get(), 1);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(projections.get(), 2);
    drop(scoped);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(projections.get(), 2);
    assert_eq!(&*mapped.get(), "2");
    assert_eq!(projections.get(), 3);
}

#[test]
fn inspection_and_emission_guards_clean_up_their_distinct_registries() {
    let store: Store<i32, (), i32> = Store::new(StoreConfig::new(0, |_, (), enqueue| {
        enqueue.emit(7);
        Ok(Update::Keep)
    }));
    let inspections = Rc::new(RefCell::new(Vec::new()));
    let inspected = inspections.clone();
    let inspection = store
        .inspect(move |record| {
            inspected
                .borrow_mut()
                .push(matches!(record, Inspection::Initial { .. }));
            Ok(())
        })
        .unwrap()
        .scoped();
    let messages = Rc::new(RefCell::new(Vec::new()));
    let observed = messages.clone();
    let emission = store
        .on_any(move |message| {
            observed.borrow_mut().push(*message);
            Ok(())
        })
        .scoped();
    store.send(()).unwrap();
    assert_eq!(*inspections.borrow(), [true, false]);
    assert_eq!(*messages.borrow(), [7]);
    drop(inspection);
    drop(emission);
    store.send(()).unwrap();
    assert_eq!(*inspections.borrow(), [true, false]);
    assert_eq!(*messages.borrow(), [7]);
}

#[test]
fn a_callback_can_drop_its_own_guard_before_a_nested_send() {
    let store = counter();
    let weak = store.downgrade();
    let holder: Rc<RefCell<Option<ScopedSubscription>>> = Rc::new(RefCell::new(None));
    let callback_holder = Rc::downgrade(&holder);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    let subscription = store.subscribe(move |snapshot| {
        observed.borrow_mut().push(snapshot.context.count);
        if let Some(holder) = callback_holder.upgrade() {
            let guard = holder.borrow_mut().take();
            drop(guard);
        }
        if let Some(store) = weak.upgrade() {
            store.send(Event::Add(1))?;
        }
        Ok(())
    });
    holder.replace(Some(subscription.scoped()));
    store.send(Event::Add(1)).unwrap();
    assert_eq!(store.get_snapshot().context.count, 2);
    assert_eq!(*calls.borrow(), [1]);
    assert!(holder.borrow().is_none());
}
