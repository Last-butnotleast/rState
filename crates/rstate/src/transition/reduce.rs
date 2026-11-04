use std::{collections::VecDeque, rc::Rc};

use crate::{Enqueue, Snapshot, StoreResult, TransitionResult, Update};

use super::Reducer;

pub fn reduce<C, E, M>(
    reducer: &Reducer<C, E, M>,
    snapshot: Rc<Snapshot<C>>,
    event: &E,
) -> StoreResult<TransitionResult<C, E, M>> {
    let mut result = TransitionResult {
        snapshot,
        effects: Vec::new(),
        allowed: false,
    };
    let mut pending = VecDeque::new();
    apply(reducer, event, &mut result, &mut pending)?;

    while let Some(event) = pending.pop_front() {
        apply(reducer, &event, &mut result, &mut pending)?;
    }

    Ok(result)
}

fn apply<C, E, M>(
    reducer: &Reducer<C, E, M>,
    event: &E,
    result: &mut TransitionResult<C, E, M>,
    pending: &mut VecDeque<E>,
) -> StoreResult<()> {
    let mut enqueue = Enqueue::default();
    let update = reducer(&result.snapshot.context, event, &mut enqueue)?;
    result.allowed |= !enqueue.events.is_empty() || !enqueue.effects.is_empty();

    if let Update::Replace(context) = update {
        result.allowed = true;
        if !Rc::ptr_eq(&result.snapshot.context, &context) {
            result.snapshot = Rc::new(Snapshot::new(context));
        }
    }

    pending.extend(enqueue.events);
    result.effects.extend(enqueue.effects);
    Ok(())
}
