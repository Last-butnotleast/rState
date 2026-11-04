use std::{cell::Cell, rc::Rc};

use gpui_kit::{AppContext, TestAppContext};
use rstate::{Store, StoreConfig, Update};
use rstate_gpui::ViewBindings;

use crate::support::{Event, counter, observe, view};

#[gpui_kit::test]
fn initial_reads_and_equal_changes_do_not_notify(cx: &mut TestAppContext) {
    let store = counter();
    let view = view(cx, &store, 1);
    let notifications = observe(cx, &view);
    let first = view.read_with(cx, |view, _| view.values[0].get());
    assert_eq!(*first, 0);
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
    store.send(Event::Set(0)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 0);
    assert!(view.read_with(cx, |view, _| Rc::ptr_eq(&first, &view.values[0].get())));
}

#[gpui_kit::test]
fn snapshot_history_binding_tracks_metadata_without_context_change(cx: &mut TestAppContext) {
    let store = counter();
    let mut history = None;
    let view = cx.new(|cx| {
        let target = ViewBindings::<crate::support::View>::new(cx);
        history = Some(target.bind(store.select_snapshot(
            |snapshot| snapshot.history_status().past != 0,
            PartialEq::eq,
        )));
        crate::support::View {
            values: vec![target.bind(store.select(|value| *value, PartialEq::eq))],
            target,
        }
    });
    let history = history.unwrap();
    let notifications = observe(cx, &view);
    assert!(!*history.get());
    store.send(Event::Keep).unwrap();
    assert!(*history.get());
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    store.undo().unwrap();
    assert!(!*history.get());
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
}

#[gpui_kit::test]
fn binding_has_no_clone_send_or_debug_bounds(cx: &mut TestAppContext) {
    struct Value(Rc<Cell<i32>>);
    struct Context(Value);
    struct LocalView {
        _target: ViewBindings<Self>,
    }
    let store = Store::new(StoreConfig::<Context, i32>::new(
        Context(Value(Rc::new(Cell::new(0)))),
        |_, event, _| Ok(Update::replace(Context(Value(Rc::new(Cell::new(*event)))))),
    ));
    let mut binding = None;
    let entity = cx.new(|cx| {
        let target = ViewBindings::<LocalView>::new(cx);
        let clone = target.clone();
        binding = Some(clone.bind(store.select(
            |context| Value(context.0.0.clone()),
            |left, right| left.0.get() == right.0.get(),
        )));
        LocalView { _target: target }
    });
    let binding = binding.unwrap();
    store.send(7).unwrap();
    assert_eq!(binding.get().0.get(), 7);
    cx.run_until_parked();
    drop(entity);
    store.send(8).unwrap();
    cx.run_until_parked();
    assert_eq!(binding.get().0.get(), 8);
}

#[gpui_kit::test]
fn mapped_and_combined_bindings_read_coherent_latest_values(cx: &mut TestAppContext) {
    let store = counter();
    let source = store.select(|value| *value, PartialEq::eq);
    let text = source.map(ToString::to_string, PartialEq::eq);
    let combined = text.combine(
        &source,
        |text, value| format!("{text}:{value}"),
        PartialEq::eq,
    );
    let mut binding = None;
    let view = cx.new(|cx| {
        let target = ViewBindings::<crate::support::View>::new(cx);
        binding = Some(target.bind(combined));
        crate::support::View {
            target,
            values: Vec::new(),
        }
    });
    let binding = binding.unwrap();
    let notifications = observe(cx, &view);
    assert_eq!(&*binding.get(), "0:0");
    store.send(Event::Set(1)).unwrap();
    store.send(Event::Set(2)).unwrap();
    assert_eq!(&*binding.get(), "2:2");
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
}

#[gpui_kit::test]
fn available_undo_routes_validation_failure_without_notifying_or_changing_identity(
    cx: &mut TestAppContext,
) {
    use rstate::{
        DispatchStatus, Schema, SchemaCheck, SchemaMetadata, Schemas, ValidationIssue,
        ValidationOptions, ValidationStage,
    };
    use rstate_gpui::handle_result;

    let minimum = Rc::new(Cell::new(0));
    let required = minimum.clone();
    let schemas = Schemas::default().with_context(Schema::new(
        SchemaMetadata::new("minimum"),
        move |value: &i32| {
            Ok(if *value < required.get() {
                SchemaCheck::invalid(vec![ValidationIssue::new("below current minimum")])
            } else {
                SchemaCheck::Valid
            })
        },
    ));
    let store = Store::new_validated(
        StoreConfig::<i32, Event>::new(0, |_, event, _| {
            Ok(match event {
                Event::Set(value) => Update::replace(*value),
                Event::Keep | Event::Fail => Update::Keep,
            })
        })
        .with_history(4)
        .with_schemas(schemas),
        ValidationOptions::default(),
    )
    .unwrap();
    let mut availability = None;
    let view = cx.new(|cx| {
        let target = ViewBindings::<crate::support::View>::new(cx);
        availability = Some(target.bind(store.select_snapshot(
            |snapshot| snapshot.history_status().past != 0,
            PartialEq::eq,
        )));
        crate::support::View {
            values: vec![target.bind(store.select(|value| *value, PartialEq::eq))],
            target,
        }
    });
    let availability = availability.unwrap();
    let notifications = observe(cx, &view);
    store.send(Event::Set(1)).unwrap();
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    let snapshot = store.get_snapshot();
    let history = store.history_status();
    let selected = view.read_with(cx, |view, _| view.values[0].get());
    let available = availability.get();
    assert!(*available);
    minimum.set(1);
    let mut errors = Vec::new();
    view.update(cx, |_, _| {
        assert!(!handle_result(store.undo(), |error| errors.push(error)));
    });
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
    let validation = error.validation_error().unwrap();
    assert_eq!(validation.stage, ValidationStage::Context);
    assert_eq!(validation.issues[0].message, "below current minimum");
    assert!(Rc::ptr_eq(&snapshot, &store.get_snapshot()));
    assert_eq!(history, store.history_status());
    assert!(Rc::ptr_eq(&available, &availability.get()));
    assert!(view.read_with(cx, |view, _| Rc::ptr_eq(&selected, &view.values[0].get())));
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
}
