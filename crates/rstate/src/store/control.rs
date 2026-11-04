use std::rc::Rc;

use crate::{ControlAction, DispatchStatus, HistoryStatus, Snapshot, StoreResult, ValidationStage};

use super::{Store, commit::Action};

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn reset(&self) -> StoreResult<()> {
        self.reset_with(|initial, _| Ok(initial.clone()))
    }

    pub fn reset_with(
        &self,
        reset: impl FnOnce(&Rc<C>, &Rc<C>) -> StoreResult<Rc<C>>,
    ) -> StoreResult<()> {
        let previous = self.get_snapshot();
        let context = reset(&self.inner.defaults, &previous.context)
            .map_err(|error| error.with_dispatch_status(DispatchStatus::NotCommitted))?;
        self.validate_context(&context, ValidationStage::Context)
            .map_err(|error| error.with_dispatch_status(DispatchStatus::NotCommitted))?;
        let snapshot = Rc::new(previous.record(context, self.inner.history_limit));
        self.commit(snapshot, &Action::Control(ControlAction::Reset), Vec::new())
    }

    pub fn history_status(&self) -> HistoryStatus {
        self.get_snapshot().history_status()
    }

    pub fn can_undo(&self) -> bool {
        self.history_status().past != 0
    }

    pub fn can_redo(&self) -> bool {
        self.history_status().future != 0
    }

    pub fn undo(&self) -> StoreResult<()> {
        self.restore(ControlAction::Undo)
    }

    pub fn redo(&self) -> StoreResult<()> {
        self.restore(ControlAction::Redo)
    }

    fn restore(&self, action: ControlAction) -> StoreResult<()> {
        let current = self.get_snapshot();
        let restored = current.history.as_ref().and_then(|history| match action {
            ControlAction::Undo => history.undo(current.context.clone()),
            ControlAction::Redo => history.redo(current.context.clone()),
            ControlAction::Reset => None,
        });
        let snapshot = restored.map_or(current, |(context, history)| {
            Rc::new(Snapshot {
                context,
                history: Some(Rc::new(history)),
            })
        });
        self.validate_context(&snapshot.context, ValidationStage::Context)
            .map_err(|error| error.with_dispatch_status(DispatchStatus::NotCommitted))?;
        self.commit(snapshot, &Action::Control(action), Vec::new())
    }
}
