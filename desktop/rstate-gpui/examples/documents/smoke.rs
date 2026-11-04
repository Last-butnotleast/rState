use std::time::Duration;

use gpui_kit::{App, AsyncApp};

use super::view::{Action, DocumentWindows};

#[expect(
    clippy::future_not_send,
    reason = "native smoke actions and Rc-backed stores remain on GPUI's foreground executor"
)]
pub async fn run(windows: DocumentWindows, cx: &mut AsyncApp) -> Result<(), String> {
    cx.background_executor()
        .timer(Duration::from_millis(250))
        .await;
    let initial_renders = cx.update(|cx| initial_render_count(&windows, cx))?;

    windows
        .list
        .update(cx, |view, _, cx| view.perform(Action::Rename, cx))
        .map_err(|error| error.to_string())?;
    cx.background_executor()
        .timer(Duration::from_millis(150))
        .await;
    cx.update(|cx| verify_shared_rename(&windows, initial_renders, cx))?;

    for action in [Action::Delete, Action::Undo, Action::Redo, Action::Reset] {
        windows
            .list
            .update(cx, |view, _, cx| view.perform(action, cx))
            .map_err(|error| error.to_string())?;
    }
    let weak = cx.update(|cx| {
        let detail = windows.detail.read(cx).map_err(|error| error.to_string())?;
        require(
            detail.row_count() == 3,
            "Reset did not restore document rows",
        )?;
        require(
            detail.selected_title().as_deref() == Some("First draft"),
            "Reset did not restore selected detail",
        )?;
        require(
            detail.error().is_none(),
            "A domain action returned an error",
        )?;
        Ok::<_, String>(detail.store().downgrade())
    })?;
    windows
        .list
        .update(cx, |_, window, _| window.remove_window())
        .map_err(|error| error.to_string())?;
    windows
        .detail
        .update(cx, |view, _, cx| view.perform(Action::Rename, cx))
        .map_err(|error| error.to_string())?;
    cx.background_executor()
        .timer(Duration::from_millis(150))
        .await;
    cx.update(|cx| {
        let detail = windows.detail.read(cx).map_err(|error| error.to_string())?;
        require(
            detail.selected_title().as_deref() == Some("First draft *"),
            "Closing the list disabled the detail window",
        )?;
        require(
            weak.upgrade().is_some(),
            "Shared document released while its detail window remained",
        )
    })?;
    windows
        .detail
        .update(cx, |_, window, _| window.remove_window())
        .map_err(|error| error.to_string())?;
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    require(
        weak.upgrade().is_none(),
        "Shared document leaked after both windows closed",
    )?;
    cx.update(|cx| {
        let independent = windows
            .independent
            .read(cx)
            .map_err(|error| error.to_string())?;
        require(
            independent.row_count() == 3,
            "Independent document did not survive shared document closure",
        )
    })?;
    println!(
        "Document smoke passed: three native windows rendered; shared rename/delete/history/reset and independent ownership verified"
    );
    Ok(())
}

fn initial_render_count(windows: &DocumentWindows, cx: &App) -> Result<usize, String> {
    let list = windows.list.read(cx).map_err(|error| error.to_string())?;
    let detail = windows.detail.read(cx).map_err(|error| error.to_string())?;
    let independent = windows
        .independent
        .read(cx)
        .map_err(|error| error.to_string())?;
    require(list.render_count() > 0, "Shared list did not render")?;
    require(detail.render_count() > 0, "Shared detail did not render")?;
    require(
        independent.render_count() > 0,
        "Independent document did not render",
    )?;
    Ok(detail.render_count())
}

fn verify_shared_rename(
    windows: &DocumentWindows,
    initial_renders: usize,
    cx: &App,
) -> Result<(), String> {
    let detail = windows.detail.read(cx).map_err(|error| error.to_string())?;
    let independent = windows
        .independent
        .read(cx)
        .map_err(|error| error.to_string())?;
    require(
        detail.selected_title().as_deref() == Some("First draft *"),
        "Rename did not reach shared detail",
    )?;
    require(
        detail.render_count() > initial_renders,
        "Shared detail did not render its changed binding",
    )?;
    require(
        independent.selected_title().as_deref() == Some("First draft"),
        "Independent document changed",
    )
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
