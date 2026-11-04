use rstate::{Store, StoreError, StoreFactory, StoreResult, Update};

pub type DocumentStore = Store<Document, Event>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: u64,
    pub title: String,
}

#[derive(Debug)]
pub struct Document {
    pub name: String,
    pub rows: Vec<Row>,
    pub selected: Option<u64>,
}

impl Document {
    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.iter().find(|row| Some(row.id) == self.selected)
    }
}

#[derive(Debug)]
pub enum Event {
    Select(u64),
    Rename { id: u64, title: String },
    Delete(u64),
}

pub fn factory() -> StoreFactory<String, Document, Event> {
    StoreFactory::new(
        |name| {
            Ok(Document {
                name,
                rows: vec![
                    Row {
                        id: 1,
                        title: "First draft".into(),
                    },
                    Row {
                        id: 2,
                        title: "Research notes".into(),
                    },
                    Row {
                        id: 3,
                        title: "Release checklist".into(),
                    },
                ],
                selected: Some(1),
            })
        },
        |document, event, _| reduce(document, event),
    )
    .with_history(32)
}

fn reduce(document: &Document, event: &Event) -> StoreResult<Update<Document>> {
    let id = match event {
        Event::Select(id) | Event::Delete(id) | Event::Rename { id, .. } => *id,
    };
    if !document.rows.iter().any(|row| row.id == id) {
        return Err(StoreError::new(format!("Item {id} no longer exists")));
    }

    let mut rows = document.rows.clone();
    let mut selected = document.selected;
    match event {
        Event::Select(_) if selected == Some(id) => return Ok(Update::Keep),
        Event::Select(_) => selected = Some(id),
        Event::Rename { title, .. } => {
            if title.trim().is_empty() {
                return Err(StoreError::new("An item title cannot be empty"));
            }
            for row in &mut rows {
                if row.id == id {
                    row.title.clone_from(title);
                }
            }
        }
        Event::Delete(_) => {
            rows.retain(|row| row.id != id);
            if selected == Some(id) {
                selected = rows.first().map(|row| row.id);
            }
        }
    }
    Ok(Update::replace(Document {
        name: document.name.clone(),
        rows,
        selected,
    }))
}
