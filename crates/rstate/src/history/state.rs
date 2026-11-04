use std::{collections::VecDeque, rc::Rc};

use super::HistoryStatus;

#[derive(Debug)]
pub struct History<C> {
    past: VecDeque<Rc<C>>,
    future: Vec<Rc<C>>,
    limit: usize,
}

impl<C> Clone for History<C> {
    fn clone(&self) -> Self {
        Self {
            past: self.past.clone(),
            future: self.future.clone(),
            limit: self.limit,
        }
    }
}

impl<C> History<C> {
    pub(crate) const fn new(limit: usize) -> Self {
        Self {
            past: VecDeque::new(),
            future: Vec::new(),
            limit,
        }
    }

    pub(crate) fn status(&self) -> HistoryStatus {
        HistoryStatus {
            past: self.past.len(),
            future: self.future.len(),
            limit: Some(self.limit),
        }
    }

    pub(crate) fn record(&self, context: Rc<C>) -> Self {
        let mut history = self.clone();
        history.push_past(context);
        history.future.clear();
        history
    }

    pub(crate) fn undo(&self, current: Rc<C>) -> Option<(Rc<C>, Self)> {
        let mut history = self.clone();
        let context = history.past.pop_back()?;
        history.future.push(current);
        Some((context, history))
    }

    pub(crate) fn redo(&self, current: Rc<C>) -> Option<(Rc<C>, Self)> {
        let mut history = self.clone();
        let context = history.future.pop()?;
        history.push_past(current);
        Some((context, history))
    }

    fn push_past(&mut self, context: Rc<C>) {
        if self.limit == 0 {
            return;
        }
        if self.past.len() == self.limit {
            self.past.pop_front();
        }
        self.past.push_back(context);
    }
}
