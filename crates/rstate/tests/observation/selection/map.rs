use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{Selection, Store, StoreConfig, StoreError, Subscription, Update};

use super::{Event, Selected, store};

struct Bucket {
    value: i32,
}

fn record(
    selection: &Selection<i32>,
    name: &'static str,
    trace: &Rc<RefCell<Vec<(&'static str, i32)>>>,
) -> Subscription {
    let trace = trace.clone();
    selection.subscribe(move |value| {
        trace.borrow_mut().push((name, *value));
        Ok(())
    })
}

#[test]
fn prior_reads_order_chained_callbacks_and_detach_resets_dependency_positions() {
    let store = store();
    let source = store.select(|context| context.count, |left, right| left == right);
    let mapped = source.map(|value| *value, |left, right| left == right);
    let chained = mapped.map(|value| *value, |left, right| left == right);
    let trace = Rc::new(RefCell::new(Vec::new()));

    assert_eq!(*chained.get(), 0);
    let first = record(&mapped, "a", &trace);
    let second = record(&mapped, "b", &trace);
    let child = record(&chained, "c", &trace);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), [("c", 1), ("a", 1), ("b", 1)]);

    child.unsubscribe();
    let child = record(&chained, "c", &trace);
    trace.borrow_mut().clear();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), [("a", 2), ("b", 2), ("c", 2)]);

    first.unsubscribe();
    second.unsubscribe();
    child.unsubscribe();
    assert_eq!(*chained.get(), 2);
    let first = record(&mapped, "a", &trace);
    let child = record(&chained, "c", &trace);
    trace.borrow_mut().clear();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), [("c", 3), ("a", 3)]);
    first.unsubscribe();
    child.unsubscribe();
}

#[test]
fn a_plain_mapped_read_precedes_later_direct_source_listeners() {
    let store = store();
    let source = store.select(|context| context.count, |left, right| left == right);
    let mapped = source.map(|value| *value, |left, right| left == right);
    let trace = Rc::new(RefCell::new(Vec::new()));

    assert_eq!(*mapped.get(), 0);
    let direct = record(&source, "source", &trace);
    let child = record(&mapped, "mapped", &trace);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), [("mapped", 1), ("source", 1)]);
    direct.unsubscribe();
    child.unsubscribe();
}

#[test]
fn unobserved_mapped_dependencies_preserve_source_cache_and_root_order() {
    let store = store();
    let calls = Rc::new((Cell::new(0_u32), Cell::new(0_u32)));
    let source_calls = calls.clone();
    let source = store.select(
        move |context| {
            source_calls.0.set(source_calls.0.get().saturating_add(1));
            context.count
        },
        |left, right| left == right,
    );
    let mapped_calls = calls.clone();
    let mapped = source.map(
        move |value| {
            mapped_calls.1.set(mapped_calls.1.get().saturating_add(1));
            *value
        },
        |left, right| left == right,
    );
    let initial = mapped.get();
    let source_subscription = source.subscribe(|_| Ok(()));
    source_subscription.unsubscribe();
    assert!(Rc::ptr_eq(&initial, &mapped.get()));
    assert_eq!((calls.0.get(), calls.1.get()), (1, 1));

    let trace = Rc::new(RefCell::new(Vec::new()));
    let direct_trace = trace.clone();
    let direct = store.subscribe(move |snapshot| {
        direct_trace
            .borrow_mut()
            .push(("direct", snapshot.context.count));
        Ok(())
    });
    let mapped_subscription = record(&mapped, "mapped", &trace);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), [("mapped", 1), ("direct", 1)]);
    assert_eq!((calls.0.get(), calls.1.get()), (2, 2));
    direct.unsubscribe();
    mapped_subscription.unsubscribe();
}

#[test]
fn detach_recomputes_a_read_chain_once_without_replacing_equal_values() {
    let store = store();
    let calls = Rc::new([Cell::new(0_u32), Cell::new(0_u32), Cell::new(0_u32)]);
    let source_calls = calls.clone();
    let source = store.select(
        move |context| {
            source_calls[0].set(source_calls[0].get().saturating_add(1));
            context.count
        },
        |left, right| left == right,
    );
    let mapped_calls = calls.clone();
    let mapped = source.map(
        move |value| {
            mapped_calls[1].set(mapped_calls[1].get().saturating_add(1));
            *value
        },
        |left, right| left == right,
    );
    let chain_calls = calls.clone();
    let chained = mapped.map(
        move |value| {
            chain_calls[2].set(chain_calls[2].get().saturating_add(1));
            *value
        },
        |left, right| left == right,
    );
    let first = chained.get();
    let subscription = chained.subscribe(|_| Ok(()));
    assert_eq!(calls.each_ref().map(Cell::get), [1, 1, 1]);
    subscription.unsubscribe();
    assert!(Rc::ptr_eq(&first, &chained.get()));
    assert_eq!(calls.each_ref().map(Cell::get), [2, 2, 2]);
    assert!(Rc::ptr_eq(&first, &chained.get()));
    assert_eq!(calls.each_ref().map(Cell::get), [2, 2, 2]);
}

#[test]
fn active_chains_do_not_retain_a_store_after_every_handle_is_dropped() {
    let lifetime = Rc::new(());
    let weak = Rc::downgrade(&lifetime);
    let store = Store::<i32, ()>::new(StoreConfig::new(0, move |_, (), _| {
        #[expect(
            clippy::no_effect_underscore_binding,
            reason = "the captured marker verifies reducer lifetime without changing state"
        )]
        let _retained = &lifetime;
        Ok(Update::Keep)
    }));
    let source = store.select(|value| *value, |left, right| left == right);
    let mapped = source.map(|value| *value, |left, right| left == right);
    let chained = mapped.map(|value| *value, |left, right| left == right);
    let subscription = chained.subscribe(|_| Ok(()));
    drop(subscription);
    drop(chained);
    drop(mapped);
    drop(source);
    drop(store);
    assert!(weak.upgrade().is_none());
}

#[test]
fn mapped_values_are_lazy_cached_and_keep_equal_output_identity() {
    let store = store();
    let source_calls = Rc::new(Cell::new(0));
    let source_computations = source_calls.clone();
    let source = store.select(
        move |context| {
            source_computations.set(source_computations.get() + 1);
            Selected {
                count: context.count,
            }
        },
        |left, right| left.count == right.count,
    );
    let map_calls = Rc::new(Cell::new(0));
    let computations = map_calls.clone();
    let mapped = source.map(
        move |selected| {
            computations.set(computations.get() + 1);
            Bucket {
                value: selected.count % 2,
            }
        },
        |left, right| left.value == right.value,
    );

    assert!(format!("{mapped:?}").contains("Selection"));
    assert_eq!((source_calls.get(), map_calls.get()), (0, 0));
    let first = mapped.get();
    assert_eq!(first.value, 0);
    assert!(Rc::ptr_eq(&first, &mapped.get()));
    assert_eq!((source_calls.get(), map_calls.get()), (1, 1));

    store.send(Event::Label).unwrap();
    assert_eq!((source_calls.get(), map_calls.get()), (1, 1));
    assert!(Rc::ptr_eq(&first, &mapped.get()));
    assert_eq!((source_calls.get(), map_calls.get()), (2, 1));

    store.send(Event::Add(2)).unwrap();
    assert!(Rc::ptr_eq(&first, &mapped.get()));
    assert_eq!((source_calls.get(), map_calls.get()), (3, 2));
    store.send(Event::Add(1)).unwrap();
    let changed = mapped.get();
    assert_eq!(changed.value, 1);
    assert!(!Rc::ptr_eq(&first, &changed));
    assert_eq!((source_calls.get(), map_calls.get()), (4, 3));
}

#[test]
fn mapped_listeners_share_computation_detach_and_reactivate_silently() {
    let store = store();
    let calls = Rc::new(Cell::new(0));
    let computations = calls.clone();
    let source = store.select(|context| context.count, |left, right| left == right);
    let mapped = source.map(
        move |count| {
            computations.set(computations.get() + 1);
            *count * 10
        },
        |left, right| left == right,
    );
    let values = Rc::new(RefCell::new(Vec::new()));
    let first_values = values.clone();
    let first = mapped.subscribe(move |value| {
        first_values.borrow_mut().push(("a", *value));
        Ok(())
    });
    let second_values = values.clone();
    let second = mapped.subscribe(move |value| {
        second_values.borrow_mut().push(("b", *value));
        Ok(())
    });
    assert_eq!(calls.get(), 1);
    assert!(values.borrow().is_empty());

    store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(*values.borrow(), [("a", 10), ("b", 10)]);
    first.unsubscribe();
    first.unsubscribe();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(*values.borrow(), [("a", 10), ("b", 10), ("b", 20)]);
    second.unsubscribe();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(*mapped.get(), 30);
    assert_eq!(calls.get(), 4);

    let third_values = values.clone();
    let third = mapped.subscribe(move |value| {
        third_values.borrow_mut().push(("c", *value));
        Ok(())
    });
    assert_eq!(calls.get(), 4);
    assert_eq!(values.borrow().len(), 3);
    store.send(Event::Label).unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(values.borrow().len(), 3);
    store.send(Event::Add(1)).unwrap();
    assert_eq!(calls.get(), 5);
    assert_eq!(values.borrow().last(), Some(&("c", 40)));
    third.unsubscribe();
}

#[test]
fn mapped_registrations_survive_dropped_selection_and_subscription_handles() {
    let store = store();
    let source = store.select(|context| context.count, |left, right| left == right);
    let mapped = source.map(|count| *count * 10, |left, right| left == right);
    let values = Rc::new(RefCell::new(Vec::new()));
    let dropped_values = values.clone();
    let dropped = mapped.subscribe(move |value| {
        dropped_values.borrow_mut().push(("dropped", *value));
        Ok(())
    });
    let retained_values = values.clone();
    let retained = mapped.subscribe(move |value| {
        retained_values.borrow_mut().push(("retained", *value));
        Ok(())
    });
    drop(dropped);
    drop(mapped);
    drop(source);

    store.send(Event::Add(1)).unwrap();
    assert_eq!(*values.borrow(), [("dropped", 10), ("retained", 10)]);
    retained.unsubscribe();
    store.send(Event::Add(1)).unwrap();
    assert_eq!(values.borrow().last(), Some(&("dropped", 20)));
    assert_eq!(values.borrow().len(), 3);
}

#[test]
fn mapped_chains_retain_the_source_until_all_handles_are_released() {
    let lifetime = Rc::new(());
    let weak = Rc::downgrade(&lifetime);
    let store = Store::<i32, i32>::new(StoreConfig::new(
        1,
        move |context: &Rc<i32>, by: &i32, _| {
            #[expect(
                clippy::no_effect_underscore_binding,
                reason = "the captured marker verifies reducer lifetime without changing state"
            )]
            let _retained = &lifetime;
            Ok(Update::replace(**context + by))
        },
    ));
    let source = store.select(|value| *value, |left, right| left == right);
    let mapped = source.map(|value| *value * 2, |left, right| left == right);
    let chained = mapped.map(
        |value| format!("value:{value}"),
        |left, right| left == right,
    );
    let observed = chained.subscribe(|_| Ok(()));
    store.send(2).unwrap();
    observed.unsubscribe();
    drop(source);
    drop(mapped);
    drop(store);

    assert!(weak.upgrade().is_some());
    assert_eq!(chained.get().as_str(), "value:6");
    drop(chained);
    drop(observed);
    assert!(weak.upgrade().is_none());
}

#[test]
fn mapped_chains_stop_recomputing_after_equal_intermediate_results() {
    let store = store();
    let source = store.select(|context| context.count, |left, right| left == right);
    let bucket_calls = Rc::new(Cell::new(0));
    let bucket_computations = bucket_calls.clone();
    let bucket = source.map(
        move |count| {
            bucket_computations.set(bucket_computations.get() + 1);
            Bucket { value: count / 2 }
        },
        |left, right| left.value == right.value,
    );
    let label_calls = Rc::new(Cell::new(0));
    let label_computations = label_calls.clone();
    let label = bucket.map(
        move |value| {
            label_computations.set(label_computations.get() + 1);
            format!("bucket:{}", value.value)
        },
        |left, right| left == right,
    );
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = values.clone();
    let subscription = label.subscribe(move |value| {
        observed.borrow_mut().push(value);
        Ok(())
    });
    assert_eq!((bucket_calls.get(), label_calls.get()), (1, 1));

    store.send(Event::Add(1)).unwrap();
    assert_eq!((bucket_calls.get(), label_calls.get()), (2, 1));
    assert!(values.borrow().is_empty());
    store.send(Event::Add(1)).unwrap();
    assert_eq!((bucket_calls.get(), label_calls.get()), (3, 2));
    assert_eq!(values.borrow()[0].as_str(), "bucket:1");
    subscription.unsubscribe();
    store.send(Event::Add(2)).unwrap();
    assert_eq!((bucket_calls.get(), label_calls.get()), (3, 2));
    assert_eq!(label.get().as_str(), "bucket:2");
    assert_eq!((bucket_calls.get(), label_calls.get()), (4, 3));
}

#[test]
fn mapped_callbacks_reconcile_before_nested_send_returns() {
    let store = store();
    let source = store.select(|context| context.count, |left, right| left == right);
    let mapped = source.map(|count| *count * 10, |left, right| left == right);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let first_trace = trace.clone();
    let nested = store.clone();
    let read = mapped.clone();
    let first = mapped.subscribe(move |value| {
        first_trace.borrow_mut().push(format!("a:{value}"));
        if *value == 10 {
            nested.send(Event::Add(1))?;
            first_trace
                .borrow_mut()
                .push(format!("a-return:{}", read.get()));
        }
        Ok(())
    });
    let second_trace = trace.clone();
    let second = mapped.subscribe(move |value| {
        second_trace.borrow_mut().push(format!("b:{value}"));
        Ok(())
    });
    let direct_trace = trace.clone();
    let direct = store.subscribe(move |snapshot| {
        direct_trace
            .borrow_mut()
            .push(format!("direct:{}", snapshot.context.count));
        Ok(())
    });

    store.send(Event::Add(1)).unwrap();
    assert_eq!(*trace.borrow(), ["a:10", "b:20", "direct:2", "a-return:20"]);
    assert_eq!(*mapped.get(), 20);
    first.unsubscribe();
    second.unsubscribe();
    direct.unsubscribe();
}

#[test]
fn mapped_callbacks_drain_cross_store_notifications_synchronously() {
    let left = store();
    let right = store();
    let left_mapped = left
        .select(|context| context.count, |a, b| a == b)
        .map(|count| *count * 10, |a, b| a == b);
    let right_mapped = right
        .select(|context| context.count, |a, b| a == b)
        .map(|count| *count * 10, |a, b| a == b);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let nested = right;
    let first_trace = trace.clone();
    let first = left_mapped.subscribe(move |value| {
        first_trace.borrow_mut().push(format!("left-a:{value}"));
        nested.send(Event::Add(1))?;
        first_trace.borrow_mut().push(format!(
            "left-return:{}",
            nested.get_snapshot().context.count
        ));
        Ok(())
    });
    let second_trace = trace.clone();
    let second = left_mapped.subscribe(move |value| {
        second_trace.borrow_mut().push(format!("left-b:{value}"));
        Ok(())
    });
    let third_trace = trace.clone();
    let third = right_mapped.subscribe(move |value| {
        third_trace.borrow_mut().push(format!("right-a:{value}"));
        Ok(())
    });
    let fourth_trace = trace.clone();
    let fourth = right_mapped.subscribe(move |value| {
        fourth_trace.borrow_mut().push(format!("right-b:{value}"));
        Ok(())
    });

    left.send(Event::Add(1)).unwrap();
    assert_eq!(
        *trace.borrow(),
        [
            "left-a:10",
            "left-b:10",
            "right-a:10",
            "right-b:10",
            "left-return:1"
        ]
    );
    first.unsubscribe();
    second.unsubscribe();
    third.unsubscribe();
    fourth.unsubscribe();
}

#[test]
fn mapped_callback_errors_drain_observers_keep_state_and_skip_effects() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let effect_trace = trace.clone();
    let store = Store::<i32, ()>::new(StoreConfig::new(
        0,
        move |context: &Rc<i32>, (), enqueue| {
            let effect_trace = effect_trace.clone();
            enqueue.effect(move |_| {
                effect_trace.borrow_mut().push("effect".to_string());
                Ok(())
            });
            Ok(Update::replace(**context + 1))
        },
    ));
    let mapped = store
        .select(|value| *value, |left, right| left == right)
        .map(|value| *value * 10, |left, right| left == right);
    let first_trace = trace.clone();
    let first = mapped.subscribe(move |value| {
        first_trace.borrow_mut().push(format!("bad:{value}"));
        Err(StoreError::new("mapped failed"))
    });
    let second_trace = trace.clone();
    let second = mapped.subscribe(move |value| {
        second_trace.borrow_mut().push(format!("good:{value}"));
        Ok(())
    });
    let direct_trace = trace.clone();
    let direct = store.subscribe(move |snapshot| {
        direct_trace
            .borrow_mut()
            .push(format!("direct:{}", snapshot.context));
        Ok(())
    });

    let error = store.send(()).unwrap_err();
    assert_eq!(error.to_string(), "mapped failed");
    assert_eq!(*trace.borrow(), ["bad:10", "good:10", "direct:1"]);
    assert_eq!(*store.get_snapshot().context, 1);
    first.unsubscribe();
    store.send(()).unwrap();
    assert_eq!(
        *trace.borrow(),
        [
            "bad:10", "good:10", "direct:1", "good:20", "direct:2", "effect"
        ]
    );
    second.unsubscribe();
    direct.unsubscribe();
}
