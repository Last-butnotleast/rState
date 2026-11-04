use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{Store, StoreConfig, Update};

struct Context {
    count: u32,
    local: Rc<()>,
}

enum Event {
    Add(u32),
    Keep,
}

struct Selected {
    value: usize,
    local: Rc<()>,
}

fn store() -> Store<Context, Event> {
    Store::new(
        StoreConfig::new(
            Context {
                count: 0,
                local: Rc::new(()),
            },
            |context: &Rc<Context>, event: &Event, _| match event {
                Event::Add(by) => Ok(Update::replace(Context {
                    count: context.count.saturating_add(*by),
                    local: context.local.clone(),
                })),
                Event::Keep => Ok(Update::Keep),
            },
        )
        .with_history(5),
    )
}

#[test]
fn history_only_changes_notify_snapshot_selections_and_preserve_equal_context_values() {
    let store = store();
    let available = store.select_snapshot(
        |snapshot| snapshot.history_status().past > 0,
        |left, right| left == right,
    );
    let count = store.select(|context| context.count, |left, right| left == right);
    let initial = store.get_snapshot();
    let initial_count = count.get();
    let initial_available = available.get();
    let availability = Rc::new(RefCell::new(Vec::new()));
    let observed = availability.clone();
    let _availability = available
        .subscribe(move |value| {
            observed.borrow_mut().push(*value);
            Ok(())
        })
        .scoped();
    let counts = Rc::new(RefCell::new(Vec::new()));
    let observed = counts.clone();
    let _counts = count
        .subscribe(move |value| {
            observed.borrow_mut().push(*value);
            Ok(())
        })
        .scoped();

    assert!(availability.borrow().is_empty());
    store.send(Event::Keep).unwrap();
    let current = store.get_snapshot();
    let can_undo = available.get();
    assert!(!Rc::ptr_eq(&initial, &current));
    assert!(Rc::ptr_eq(&initial.context, &current.context));
    assert!(!Rc::ptr_eq(&initial_available, &can_undo));
    assert!(Rc::ptr_eq(&initial_count, &count.get()));
    assert_eq!(*availability.borrow(), [true]);

    store.send(Event::Keep).unwrap();
    assert!(Rc::ptr_eq(&can_undo, &available.get()));
    store.undo().unwrap();
    store.undo().unwrap();
    store.redo().unwrap();
    assert_eq!(*availability.borrow(), [true, false, true]);
    assert!(counts.borrow().is_empty());
    assert!(Rc::ptr_eq(&initial_count, &count.get()));
}

#[test]
fn snapshot_reads_are_lazy_and_bound_free_with_comparator_controlled_identity() {
    let store = store();
    let calls = Rc::new(Cell::new(0_u32));
    let computations = calls.clone();
    let selection = store.select_snapshot(
        move |snapshot| {
            computations.set(computations.get().saturating_add(1));
            Selected {
                value: snapshot.history_status().past.min(1),
                local: snapshot.context.local.clone(),
            }
        },
        |left, right| left.value == right.value,
    );
    let clone = selection.clone();

    assert!(format!("{selection:?}").contains("Selection"));
    assert_eq!(calls.get(), 0);
    let initial = selection.get();
    assert_eq!(initial.value, 0);
    assert!(Rc::ptr_eq(&initial, &clone.get()));
    assert_eq!(calls.get(), 1);

    store.send(Event::Keep).unwrap();
    assert_eq!(calls.get(), 1);
    let changed = selection.get();
    assert_eq!(changed.value, 1);
    assert!(!Rc::ptr_eq(&initial, &changed));
    assert!(Rc::ptr_eq(&changed.local, &initial.local));
    store.send(Event::Keep).unwrap();
    assert_eq!(calls.get(), 2);
    assert!(Rc::ptr_eq(&changed, &selection.get()));
    assert_eq!(calls.get(), 3);
}

#[test]
fn snapshot_mapping_combines_coherently_with_context_and_detaches_scoped_observation() {
    let store = store();
    let history = store.select_snapshot(
        |snapshot| snapshot.history_status().past,
        |left, right| left == right,
    );
    let mapped = history.map(|past| past.saturating_mul(2), |left, right| left == right);
    let count = store.select(|context| context.count, |left, right| left == right);
    let calls = Rc::new(Cell::new(0_u32));
    let computations = calls.clone();
    let combined = mapped.combine(
        &count,
        move |past, count| {
            computations.set(computations.get().saturating_add(1));
            (*count, *past)
        },
        |left, right| left == right,
    );
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = values.clone();
    let guard = combined
        .subscribe(move |value| {
            observed.borrow_mut().push(*value);
            Ok(())
        })
        .scoped();

    assert_eq!(*combined.get(), (0, 0));
    store.send(Event::Add(2)).unwrap();
    store.send(Event::Keep).unwrap();
    store.undo().unwrap();
    assert_eq!(*values.borrow(), [(2, 2), (2, 4), (2, 2)]);
    assert_eq!(calls.get(), 4);

    drop(guard);
    store.send(Event::Keep).unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(values.borrow().len(), 3);
    assert_eq!(*combined.get(), (2, 4));
    assert_eq!(calls.get(), 5);

    let observed = values.clone();
    let _guard = combined
        .subscribe(move |value| {
            observed.borrow_mut().push(*value);
            Ok(())
        })
        .scoped();
    assert_eq!(values.borrow().len(), 3);
    store.send(Event::Add(3)).unwrap();
    assert_eq!(values.borrow().last(), Some(&(5, 6)));
}

#[test]
fn snapshot_callbacks_reconcile_nested_sends_before_returning() {
    let store = store();
    let selected = store.select_snapshot(
        |snapshot| (snapshot.context.count, snapshot.history_status().past),
        |left, right| left == right,
    );
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let weak = store.downgrade();
    let current = selected.clone();
    let guard = selected
        .subscribe(move |value| {
            observed.borrow_mut().push(("selected", value.0, value.1));
            if value.0 == 1 {
                weak.upgrade().unwrap().send(Event::Add(1))?;
                let value = current.get();
                observed.borrow_mut().push(("return", value.0, value.1));
            }
            Ok(())
        })
        .scoped();
    let observed = trace.clone();
    let _direct = store
        .subscribe(move |snapshot| {
            observed.borrow_mut().push((
                "direct",
                snapshot.context.count,
                snapshot.history_status().past,
            ));
            Ok(())
        })
        .scoped();

    store.send(Event::Add(1)).unwrap();
    assert_eq!(
        *trace.borrow(),
        [("selected", 1, 1), ("direct", 2, 2), ("return", 2, 2)]
    );
    assert_eq!(*selected.get(), (2, 2));
    drop(guard);
}

#[test]
fn snapshot_selection_owns_its_store_without_subscription_cycles() {
    let store = store();
    let weak = store.downgrade();
    let selection = store.select_snapshot(
        |snapshot| snapshot.history_status().past,
        |left, right| left == right,
    );
    let guard = selection.subscribe(|_| Ok(())).scoped();
    store.send(Event::Keep).unwrap();
    drop(store);
    assert!(weak.upgrade().is_some());
    assert_eq!(*selection.get(), 1);
    drop(selection);
    assert!(weak.upgrade().is_none());
    drop(guard);
}
