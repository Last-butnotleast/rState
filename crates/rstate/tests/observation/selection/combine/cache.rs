use std::{cell::Cell, rc::Rc};

use super::{Event, selected, store};

struct Value {
    amount: i32,
}

#[test]
fn combined_values_are_lazy_bound_free_and_retain_equal_output_identity() {
    let left_store = store();
    let right_store = store();
    let left = left_store.select(
        |context| Value {
            amount: context.count,
        },
        |left, right| left.amount == right.amount,
    );
    let right = right_store.select(
        |context| Value {
            amount: context.count,
        },
        |left, right| left.amount == right.amount,
    );
    let calls = Rc::new(Cell::new(0_usize));
    let projected = calls.clone();
    let combined = left.combine(
        &right,
        move |left, right| {
            projected.set(projected.get().saturating_add(1));
            Value {
                amount: left.amount.saturating_add(right.amount).rem_euclid(2),
            }
        },
        |left, right| left.amount == right.amount,
    );

    assert!(format!("{combined:?}").contains("Selection"));
    assert_eq!(calls.get(), 0);
    let first = combined.get();
    assert!(Rc::ptr_eq(&first, &combined.get()));
    assert_eq!(calls.get(), 1);

    left_store.send(Event::Add(2)).unwrap();
    assert_eq!(calls.get(), 1);
    assert!(Rc::ptr_eq(&first, &combined.get()));
    assert_eq!(calls.get(), 2);

    right_store.send(Event::Add(1)).unwrap();
    let changed = combined.get();
    assert_eq!(changed.amount, 1);
    assert!(!Rc::ptr_eq(&first, &changed));
    assert_eq!(calls.get(), 3);
    right_store.send(Event::Label).unwrap();
    assert!(Rc::ptr_eq(&changed, &combined.get()));
    assert_eq!(calls.get(), 3);
}

#[test]
fn same_store_diamonds_publish_one_coherent_final_value() {
    let store = store();
    let source = selected(&store);
    let left = source.map(|value| value.saturating_mul(2), |left, right| left == right);
    let right = source.map(|value| value.saturating_add(1), |left, right| left == right);
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
    let values = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = values.clone();
    let subscription = combined.subscribe(move |value| {
        observed.borrow_mut().push(*value);
        Ok(())
    });

    assert_eq!(*combined.get(), 1);
    assert_eq!(calls.get(), 1);
    store.send(Event::Add(2)).unwrap();
    assert_eq!(*values.borrow(), [7]);
    assert_eq!(*combined.get(), 7);
    assert_eq!(calls.get(), 2);
    subscription.unsubscribe();
}

#[test]
fn duplicate_source_inputs_share_one_dependency_and_one_computation() {
    let store = store();
    let source_calls = Rc::new(Cell::new(0_usize));
    let reads = source_calls.clone();
    let source = store.select(
        move |context| {
            reads.set(reads.get().saturating_add(1));
            context.count
        },
        |left, right| left == right,
    );
    let calls = Rc::new(Cell::new(0_usize));
    let projected = calls.clone();
    let combined = source.combine(
        &source.clone(),
        move |left, right| {
            projected.set(projected.get().saturating_add(1));
            left.saturating_add(*right)
        },
        |left, right| left == right,
    );
    let values = Rc::new(std::cell::RefCell::new(Vec::new()));
    let first_values = values.clone();
    let first = combined.subscribe(move |value| {
        first_values.borrow_mut().push(*value);
        Ok(())
    });
    let second_values = values.clone();
    let second = combined.subscribe(move |value| {
        second_values.borrow_mut().push(*value);
        Ok(())
    });

    store.send(Event::Add(1)).unwrap();
    assert_eq!(*values.borrow(), [2, 2]);
    assert_eq!(source_calls.get(), 2);
    assert_eq!(calls.get(), 2);
    first.unsubscribe();
    second.unsubscribe();
}

#[test]
fn combine_and_map_chains_reuse_existing_selections() {
    let left_store = store();
    let right_store = store();
    let left = selected(&left_store);
    let right = selected(&right_store);
    let sum = left.combine(
        &right,
        |left, right| left.saturating_add(*right),
        |a, b| a == b,
    );
    let doubled = sum.map(|value| value.saturating_mul(2), |a, b| a == b);
    let result = doubled.combine(&left, |sum, left| sum.saturating_add(*left), |a, b| a == b);
    let values = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = values.clone();
    let subscription = result.subscribe(move |value| {
        observed.borrow_mut().push(*value);
        Ok(())
    });

    left_store.send(Event::Add(1)).unwrap();
    right_store.send(Event::Add(2)).unwrap();
    assert_eq!(*values.borrow(), [3, 7]);
    subscription.unsubscribe();
}

#[test]
fn overlapping_ancestor_inputs_are_coherent_in_both_declaration_orders() {
    for reversed in [false, true] {
        let store = store();
        let source = selected(&store);
        let derived = source.map(|value| value.saturating_mul(2), |a, b| a == b);
        let combined = if reversed {
            derived.combine(
                &source,
                |left, right| left.saturating_add(*right),
                |a, b| a == b,
            )
        } else {
            source.combine(
                &derived,
                |left, right| left.saturating_add(*right),
                |a, b| a == b,
            )
        };
        let values = Rc::new(std::cell::RefCell::new(Vec::new()));
        let observed = values.clone();
        let subscription = combined.subscribe(move |value| {
            observed.borrow_mut().push(*value);
            Ok(())
        });

        store.send(Event::Add(1)).unwrap();
        store.send(Event::Add(1)).unwrap();
        assert_eq!(*values.borrow(), [3, 6]);
        assert_eq!(*combined.get(), 6);
        subscription.unsubscribe();
    }
}
