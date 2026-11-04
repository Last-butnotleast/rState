use std::{cell::RefCell, rc::Rc};

use rstate::DispatchStatus;

use super::domain::{Event, factory};

#[test]
fn immutable_document_operations_keep_history_and_factory_instances_separate() {
    let factory = factory();
    let first = factory.create("First".into()).unwrap();
    let second = factory.create("Second".into()).unwrap();
    let initial = first.get_snapshot();
    first.send(Event::Select(2)).unwrap();
    first
        .send(Event::Rename {
            id: 2,
            title: "Revised notes".into(),
        })
        .unwrap();
    assert_eq!(
        first.get_snapshot().context.selected_row().unwrap().title,
        "Revised notes"
    );
    assert_eq!(initial.context.selected_row().unwrap().title, "First draft");
    assert_eq!(second.get_snapshot().context.selected, Some(1));
    assert_eq!(second.get_snapshot().context.name, "Second");
    first.send(Event::Delete(2)).unwrap();
    assert_eq!(first.get_snapshot().context.rows.len(), 2);
    assert_eq!(first.get_snapshot().context.selected, Some(1));
    first.undo().unwrap();
    assert_eq!(
        first.get_snapshot().context.selected_row().unwrap().title,
        "Revised notes"
    );
    first.redo().unwrap();
    assert_eq!(first.get_snapshot().context.rows.len(), 2);
    first.reset().unwrap();
    assert_eq!(first.get_snapshot().context.rows.len(), 3);
    assert_eq!(first.get_snapshot().context.selected, Some(1));
    first.undo().unwrap();
    assert_eq!(first.get_snapshot().context.rows.len(), 2);
}

#[test]
fn invalid_document_actions_report_uncommitted_errors_without_history_changes() {
    let store = factory().create("Document".into()).unwrap();
    let initial = store.get_snapshot();
    for event in [
        Event::Select(99),
        Event::Delete(99),
        Event::Rename {
            id: 1,
            title: "   ".into(),
        },
    ] {
        let error = store.send(event).unwrap_err();
        assert_eq!(error.dispatch_status(), Some(DispatchStatus::NotCommitted));
        assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    }
}

#[test]
fn same_selection_changes_history_availability_without_changing_displayed_row() {
    let store = factory().create("Document".into()).unwrap();
    let row = store.select(|document| document.selected_row().cloned(), PartialEq::eq);
    let undo = store.select_snapshot(|snapshot| snapshot.history_status().past > 0, PartialEq::eq);
    let original_row = row.get();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let changed = trace.clone();
    let _guard = undo
        .subscribe(move |value| {
            changed.borrow_mut().push(*value);
            Ok(())
        })
        .scoped();
    store.send(Event::Select(1)).unwrap();
    assert!(Rc::ptr_eq(&original_row, &row.get()));
    assert_eq!(&*trace.borrow(), &[true]);
    store.undo().unwrap();
    assert_eq!(&*trace.borrow(), &[true, false]);
}

#[test]
fn deleting_every_row_leaves_a_valid_empty_document_and_undo_restores_selection() {
    let store = factory().create("Document".into()).unwrap();
    for id in [1, 2, 3] {
        store.send(Event::Delete(id)).unwrap();
    }
    assert_eq!(store.get_snapshot().context.rows.as_slice(), &[]);
    assert!(store.get_snapshot().context.selected_row().is_none());
    store.undo().unwrap();
    assert_eq!(store.get_snapshot().context.selected_row().unwrap().id, 3);
}
