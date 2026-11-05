use std::{convert::Infallible, fmt, rc::Rc};

use crate::{Snapshot, StoreResult, Transition, TransitionOutcome};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplayPolicy {
    #[default]
    Stop,
    Skip,
}

#[non_exhaustive]
pub struct ReplayResult<C, E, M = (), R = Infallible> {
    pub snapshot: Rc<Snapshot<C>>,
    pub outcomes: Vec<TransitionOutcome<C, E, M, R>>,
}

impl<C: fmt::Debug, E, M, R: fmt::Debug> fmt::Debug for ReplayResult<C, E, M, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplayResult")
            .field("snapshot", &self.snapshot)
            .field("outcomes", &self.outcomes)
            .finish()
    }
}

pub fn replay<C, E, M>(
    transition: &Transition<C, E, M>,
    initial: Rc<Snapshot<C>>,
    events: impl IntoIterator<Item = E>,
    policy: ReplayPolicy,
) -> StoreResult<ReplayResult<C, E, M>> {
    collect(initial, events, policy, |snapshot, event| {
        transition.outcome(snapshot, event)
    })
}

pub fn replay_with_guard<C, E, M, R>(
    transition: &Transition<C, E, M>,
    initial: Rc<Snapshot<C>>,
    events: impl IntoIterator<Item = E>,
    policy: ReplayPolicy,
    mut guard: impl FnMut(&Snapshot<C>, &E) -> Result<(), R>,
) -> StoreResult<ReplayResult<C, E, M, R>> {
    collect(initial, events, policy, |snapshot, event| {
        transition.outcome_with_guard(snapshot, event, |current, event| guard(current, event))
    })
}

fn collect<C, E, M, R>(
    initial: Rc<Snapshot<C>>,
    events: impl IntoIterator<Item = E>,
    policy: ReplayPolicy,
    mut step: impl FnMut(&Rc<Snapshot<C>>, E) -> StoreResult<TransitionOutcome<C, E, M, R>>,
) -> StoreResult<ReplayResult<C, E, M, R>> {
    let mut result = ReplayResult {
        snapshot: initial,
        outcomes: Vec::new(),
    };
    for event in events {
        let outcome = step(&result.snapshot, event)?;
        let rejected = if let TransitionOutcome::Applied { result: applied } = &outcome {
            result.snapshot = applied.snapshot.clone();
            false
        } else {
            true
        };
        result.outcomes.push(outcome);
        if rejected && policy == ReplayPolicy::Stop {
            break;
        }
    }
    Ok(result)
}
