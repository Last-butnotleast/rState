use std::{fmt, rc::Rc};

use crate::{HistoryStatus, history::History};

pub struct Snapshot<C> {
    pub context: Rc<C>,
    pub(crate) history: Option<Rc<History<C>>>,
}

impl<C: fmt::Debug> fmt::Debug for Snapshot<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("context", &self.context)
            .field("history", &self.history_status())
            .finish()
    }
}

impl<C> Snapshot<C> {
    pub const fn new(context: Rc<C>) -> Self {
        Self {
            context,
            history: None,
        }
    }

    pub fn history_status(&self) -> HistoryStatus {
        self.history
            .as_ref()
            .map_or_else(HistoryStatus::default, |history| history.status())
    }

    pub(crate) fn initial(context: Rc<C>, limit: Option<usize>) -> Self {
        Self {
            context,
            history: limit.map(|limit| Rc::new(History::new(limit))),
        }
    }

    pub(crate) fn record(&self, context: Rc<C>, limit: Option<usize>) -> Self {
        Self {
            context,
            history: limit.map(|limit| {
                let history = self
                    .history
                    .as_deref()
                    .map_or_else(|| History::new(limit), Clone::clone);
                Rc::new(history.record(self.context.clone()))
            }),
        }
    }
}
