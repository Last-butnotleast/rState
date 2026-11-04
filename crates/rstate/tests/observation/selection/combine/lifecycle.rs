use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{Store, StoreConfig, Update};

use super::{Event, selected, store};

#[test]
fn listeners_share_computation_and_final_unsubscribe_restores_lazy_reads() {
    let left_store = store();
    let right_store = store();
    let left = selected(&left_store);
    let right = selected(&right_store);
    let calls = Rc::new(Cell::new(0_usize));
    let projected = calls.clone();
    let combined = left.combine(
        &right,
        move |left, right| {
            projected.set(projected.get().saturating_add(1));
            left.saturating_add(*right)
        },
        |left, right| left == right,
    );
    let first = combined.subscribe(|_| Ok(()));
    let second = combined.subscribe(|_| Ok(()));
    assert_eq!(calls.get(), 1);
    left_store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 2);
    first.unsubscribe();
    right_store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 3);
    let retained = combined.get();
    second.unsubscribe();
    assert!(Rc::ptr_eq(&retained, &combined.get()));
    assert_eq!(calls.get(), 4);
    left_store.send(Event::Add(1)).unwrap();
    right_store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(*combined.get(), 4);
    assert_eq!(calls.get(), 5);

    let replay = Rc::new(Cell::new(0_usize));
    let observed = replay.clone();
    let reattached = combined.subscribe(move |_| {
        observed.set(observed.get().saturating_add(1));
        Ok(())
    });
    assert_eq!(replay.get(), 0);
    right_store.send(Event::Add(1)).unwrap();
    assert_eq!(replay.get(), 1);
    reattached.unsubscribe();
}

#[test]
fn read_established_combined_dependencies_survive_direct_parent_unwatch() {
    let left_store = store();
    let right_store = store();
    let left = selected(&left_store);
    let right = selected(&right_store);
    let calls = Rc::new(Cell::new(0_usize));
    let projected = calls.clone();
    let combined = left.combine(
        &right,
        move |left, right| {
            projected.set(projected.get().saturating_add(1));
            left.saturating_add(*right)
        },
        |left, right| left == right,
    );
    let original = combined.get();
    let direct = left.subscribe(|_| Ok(()));
    direct.unsubscribe();
    assert!(Rc::ptr_eq(&original, &combined.get()));
    assert_eq!(calls.get(), 1);
}

struct Tracked {
    value: i32,
    dropped: Rc<Cell<usize>>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.dropped.set(self.dropped.get().saturating_add(1));
    }
}

fn tracked(dropped: Rc<Cell<usize>>) -> Store<Tracked, ()> {
    Store::new(StoreConfig::new(
        Tracked { value: 7, dropped },
        |_, (), _| Ok(Update::Keep),
    ))
}

#[test]
fn combined_handles_retain_both_sources_without_a_reference_cycle() {
    let drops = Rc::new(Cell::new(0));
    let left = tracked(drops.clone());
    let right = tracked(drops.clone());
    let combined = left.select(|context| context.value, |a, b| a == b).combine(
        &right.select(|context| context.value, |a, b| a == b),
        |left, right| left.saturating_add(*right),
        |a, b| a == b,
    );
    drop(left);
    drop(right);
    assert_eq!(drops.get(), 0);
    assert_eq!(*combined.get(), 14);
    drop(combined);
    assert_eq!(drops.get(), 2);
}

#[test]
fn surviving_callbacks_use_an_expired_inputs_last_value_without_owning_stores() {
    let left_drops = Rc::new(Cell::new(0));
    let right_drops = Rc::new(Cell::new(0));
    let left = tracked(left_drops.clone());
    let right: Store<Tracked, ()> = Store::new(StoreConfig::new(
        Tracked {
            value: 0,
            dropped: right_drops.clone(),
        },
        |context: &Rc<Tracked>, (), _| {
            Ok(Update::replace(Tracked {
                value: context.value.saturating_add(1),
                dropped: context.dropped.clone(),
            }))
        },
    ));
    let weak_left = left.downgrade();
    let weak_right = right.downgrade();
    let left_value = left.select(|context| context.value, |a, b| a == b);
    let right_value = right.select(|context| context.value, |a, b| a == b);
    let combined = left_value.combine(
        &right_value,
        |left, right| left.saturating_add(*right),
        |a, b| a == b,
    );
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = values.clone();
    let subscription = combined.subscribe(move |value| {
        observed.borrow_mut().push(*value);
        Ok(())
    });
    drop(subscription);
    drop(combined);
    drop(left_value);
    drop(right_value);
    drop(left);
    assert!(weak_left.upgrade().is_none());
    right.send(()).unwrap();
    assert_eq!(*values.borrow(), [8]);
    drop(right);
    assert!(weak_right.upgrade().is_none());
    assert_eq!(left_drops.get(), 1);
    assert_eq!(right_drops.get(), 2);
}
