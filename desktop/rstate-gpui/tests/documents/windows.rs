use gpui::TestAppContext;

use super::view::{Action, open_documents};

#[gpui::test]
fn shared_windows_render_changes_and_independent_document_survives_closure(
    cx: &mut TestAppContext,
) {
    let windows = cx.update(|cx| open_documents(cx).unwrap());
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(windows.list.read(cx).unwrap().render_count() > 0);
        assert!(windows.detail.read(cx).unwrap().render_count() > 0);
        assert!(windows.independent.read(cx).unwrap().render_count() > 0);
    });
    windows
        .list
        .update(cx, |view, _, cx| view.perform(Action::Rename, cx))
        .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(
            windows.detail.read(cx).unwrap().selected_title().as_deref(),
            Some("First draft *")
        );
        assert_eq!(
            windows
                .independent
                .read(cx)
                .unwrap()
                .selected_title()
                .as_deref(),
            Some("First draft")
        );
    });
    let weak = windows
        .detail
        .read_with(cx, |view, _| view.store().downgrade())
        .unwrap();
    windows
        .list
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    assert!(weak.upgrade().is_some());
    windows
        .detail
        .update(cx, |view, _, cx| view.perform(Action::Delete, cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        windows
            .detail
            .read_with(cx, |view, _| view.row_count())
            .unwrap(),
        2
    );
    windows
        .detail
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
    assert_eq!(
        windows
            .independent
            .read_with(cx, |view, _| view.row_count())
            .unwrap(),
        3
    );
}

#[gpui::test]
fn document_controls_restore_shared_views_and_errors_are_visible(cx: &mut TestAppContext) {
    let windows = cx.update(|cx| open_documents(cx).unwrap());
    for action in [
        Action::Rename,
        Action::Delete,
        Action::Undo,
        Action::Redo,
        Action::Reset,
    ] {
        windows
            .list
            .update(cx, |view, _, cx| view.perform(action, cx))
            .unwrap();
    }
    cx.run_until_parked();
    cx.update(|cx| {
        let detail = windows.detail.read(cx).unwrap();
        assert_eq!(detail.row_count(), 3);
        assert_eq!(detail.selected_title().as_deref(), Some("First draft"));
        assert!(detail.error().is_none());
    });
    windows
        .list
        .update(cx, |view, _, cx| view.perform(Action::Select(99), cx))
        .unwrap();
    assert!(
        windows
            .list
            .read_with(cx, |view, _| view
                .error()
                .is_some_and(|error| error.contains("Item 99")))
            .unwrap()
    );
    windows
        .list
        .update(cx, |view, _, cx| view.perform(Action::Select(1), cx))
        .unwrap();
    assert!(
        windows
            .list
            .read_with(cx, |view, _| view.error().is_none())
            .unwrap()
    );
}
