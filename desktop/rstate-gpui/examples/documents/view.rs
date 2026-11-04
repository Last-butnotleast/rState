use std::cell::Cell;

use gpui::{
    App, AppContext, Bounds, Context, IntoElement, Render, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, rgb, size,
};
use rstate::{HistoryStatus, StoreError};
use rstate_gpui::{SelectionBinding, ViewBindings, dispatch, handle_result};

use super::domain::{DocumentStore, Event, Row, factory};

#[derive(Clone, Copy)]
pub enum Action {
    Select(u64),
    Rename,
    Delete,
    Undo,
    Redo,
    Reset,
}

#[derive(Clone, Copy)]
pub enum Role {
    List,
    Detail,
}

pub struct DocumentView {
    store: DocumentStore,
    rows: SelectionBinding<Vec<Row>>,
    selected: SelectionBinding<Option<Row>>,
    history: SelectionBinding<HistoryStatus>,
    role: Role,
    name: String,
    error: Option<String>,
    renders: Cell<usize>,
}

impl DocumentView {
    pub fn new(store: DocumentStore, role: Role, cx: &Context<Self>) -> Self {
        let bindings = ViewBindings::new(cx);
        let rows = bindings.bind(store.select(|document| document.rows.clone(), PartialEq::eq));
        let selected =
            bindings.bind(store.select(|document| document.selected_row().cloned(), PartialEq::eq));
        let history =
            bindings.bind(store.select_snapshot(rstate::Snapshot::history_status, PartialEq::eq));
        let name = store.get_snapshot().context.name.clone();
        Self {
            store,
            rows,
            selected,
            history,
            role,
            name,
            error: None,
            renders: Cell::new(0),
        }
    }

    pub fn perform(&mut self, action: Action, cx: &mut Context<Self>) {
        let mut failure = None;
        let report = |error: StoreError| {
            failure = Some(format!("{error} ({:?})", error.dispatch_status()));
        };
        match action {
            Action::Select(id) => {
                dispatch(&self.store, Event::Select(id), report);
            }
            Action::Rename => match self.selected.get().as_ref() {
                Some(row) => {
                    dispatch(
                        &self.store,
                        Event::Rename {
                            id: row.id,
                            title: format!("{} *", row.title),
                        },
                        report,
                    );
                }
                None => failure = Some("Select an item before renaming".into()),
            },
            Action::Delete => match self.selected.get().as_ref() {
                Some(row) => {
                    dispatch(&self.store, Event::Delete(row.id), report);
                }
                None => failure = Some("Select an item before deleting".into()),
            },
            Action::Undo => {
                handle_result(self.store.undo(), report);
            }
            Action::Redo => {
                handle_result(self.store.redo(), report);
            }
            Action::Reset => {
                handle_result(self.store.reset(), report);
            }
        }
        if self.error != failure {
            self.error = failure;
            cx.notify();
        }
    }

    pub const fn store(&self) -> &DocumentStore {
        &self.store
    }

    pub fn selected_title(&self) -> Option<String> {
        self.selected
            .get()
            .as_ref()
            .as_ref()
            .map(|row| row.title.clone())
    }

    pub fn row_count(&self) -> usize {
        self.rows.get().len()
    }

    pub const fn render_count(&self) -> usize {
        self.renders.get()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl Render for DocumentView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        let selected = self.selected.get();
        let history = self.history.get();
        let mut content = div().flex().flex_col().gap_2();
        match self.role {
            Role::List => {
                for row in self.rows.get().iter() {
                    let id = row.id;
                    content = content.child(
                        div()
                            .id(SharedString::from(format!("row-{id}")))
                            .p_3()
                            .rounded_md()
                            .bg(rgb(
                                if selected.as_ref().as_ref().map(|row| row.id) == Some(id) {
                                    0x28_46_71
                                } else {
                                    0x1c_2b_40
                                },
                            ))
                            .cursor_pointer()
                            .child(row.title.clone())
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.perform(Action::Select(id), cx);
                            })),
                    );
                }
            }
            Role::Detail => {
                content = content
                    .child("Selected item")
                    .child(
                        div().text_xl().child(selected.as_ref().as_ref().map_or_else(
                            || "No items remain".into(),
                            |row| row.title.clone(),
                        )),
                    )
                    .child("Changes appear in both windows. Closing either window keeps the other usable.");
            }
        }

        div()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .bg(rgb(0x10_1a_2a))
            .text_color(rgb(0xe1_ea_f5))
            .child(div().text_xl().child(self.name.clone()))
            .child(match self.role {
                Role::List => "Document list",
                Role::Detail => "Document detail",
            })
            .child(content)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(button(
                        "Rename (+ *)",
                        Action::Rename,
                        selected.is_some(),
                        cx,
                    ))
                    .child(button("Delete", Action::Delete, selected.is_some(), cx))
                    .child(button("Undo", Action::Undo, history.past > 0, cx))
                    .child(button("Redo", Action::Redo, history.future > 0, cx))
                    .child(button("Reset", Action::Reset, true, cx)),
            )
            .child(format!(
                "{} items · Undo {} · Redo {}",
                self.rows.get().len(),
                history.past,
                history.future
            ))
            .child(
                div()
                    .text_color(rgb(0xfa_b9_96))
                    .child(self.error.clone().unwrap_or_default()),
            )
    }
}

fn button(
    label: &'static str,
    action: Action,
    enabled: bool,
    cx: &Context<DocumentView>,
) -> impl IntoElement {
    div()
        .id(label)
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(0x2c_43_60))
        .when(!enabled, |button| button.opacity(0.4))
        .when(enabled, gpui::Styled::cursor_pointer)
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| {
            if enabled {
                view.perform(action, cx);
            }
        }))
}

#[derive(Clone, Copy)]
pub struct DocumentWindows {
    pub list: WindowHandle<DocumentView>,
    pub detail: WindowHandle<DocumentView>,
    pub independent: WindowHandle<DocumentView>,
}

pub fn open_documents(cx: &mut App) -> Result<DocumentWindows, String> {
    let factory = factory();
    let shared = factory
        .create("Shared document".into())
        .map_err(|error| error.to_string())?;
    let independent = factory
        .create("Independent document".into())
        .map_err(|error| error.to_string())?;
    Ok(DocumentWindows {
        list: open(shared.clone(), Role::List, "rState · Shared list", cx)?,
        detail: open(shared, Role::Detail, "rState · Shared detail", cx)?,
        independent: open(independent, Role::List, "rState · Independent", cx)?,
    })
}

fn open(
    store: DocumentStore,
    role: Role,
    title: &'static str,
    cx: &mut App,
) -> Result<WindowHandle<DocumentView>, String> {
    let bounds = Bounds::centered(None, size(px(620.0), px(520.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                ..TitlebarOptions::default()
            }),
            ..WindowOptions::default()
        },
        move |_, cx| cx.new(|cx| DocumentView::new(store, role, cx)),
    )
    .map_err(|error| error.to_string())
}
