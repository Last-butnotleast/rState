use std::{convert::Infallible, fmt, rc::Rc};

use crate::{Snapshot, StoreError, StoreResult, Transition, TransitionResult};

#[non_exhaustive]
pub enum TransitionOutcome<C, E, M = (), R = Infallible> {
    Applied { result: TransitionResult<C, E, M> },
    Unavailable,
    Rejected { reason: R },
    ValidationFailed { error: StoreError },
}

impl<C: fmt::Debug, E, M, R: fmt::Debug> fmt::Debug for TransitionOutcome<C, E, M, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Applied { result } => f.debug_struct("Applied").field("result", result).finish(),
            Self::Unavailable => f.write_str("Unavailable"),
            Self::Rejected { reason } => {
                f.debug_struct("Rejected").field("reason", reason).finish()
            }
            Self::ValidationFailed { error } => f
                .debug_struct("ValidationFailed")
                .field("error", error)
                .finish(),
        }
    }
}

impl<C, E, M> Transition<C, E, M> {
    pub fn outcome(
        &self,
        snapshot: &Rc<Snapshot<C>>,
        event: E,
    ) -> StoreResult<TransitionOutcome<C, E, M>> {
        classify(self.transition(snapshot, event))
    }

    pub fn outcome_with_guard<R>(
        &self,
        snapshot: &Rc<Snapshot<C>>,
        event: E,
        guard: impl FnOnce(&Snapshot<C>, &E) -> Result<(), R>,
    ) -> StoreResult<TransitionOutcome<C, E, M, R>> {
        if let Err(reason) = guard(snapshot, &event) {
            return Ok(TransitionOutcome::Rejected { reason });
        }
        classify(self.transition(snapshot, event))
    }
}

fn classify<C, E, M, R>(
    result: StoreResult<TransitionResult<C, E, M>>,
) -> StoreResult<TransitionOutcome<C, E, M, R>> {
    match result {
        Ok(result) if result.allowed => Ok(TransitionOutcome::Applied { result }),
        Ok(_) => Ok(TransitionOutcome::Unavailable),
        Err(error) if error.validation_error().is_some() => {
            Ok(TransitionOutcome::ValidationFailed { error })
        }
        Err(error) => Err(error),
    }
}
