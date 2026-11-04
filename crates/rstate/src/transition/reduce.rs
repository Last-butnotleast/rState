use std::{collections::VecDeque, rc::Rc};

use crate::{Enqueue, Snapshot, StoreResult, TransitionResult, Update};

use super::Reducer;

pub(crate) fn reduce<C, E, M>(
    reducer: &Reducer<C, E, M>,
    snapshot: Rc<Snapshot<C>>,
    event: E,
) -> StoreResult<TransitionResult<C, E, M>> {
    let mut snapshot = snapshot;
    let mut pending = VecDeque::from([event]);
    let mut effects = Vec::new();
    let mut allowed = false;

    while let Some(event) = pending.pop_front() {
        let mut enqueue = Enqueue::default();
        let update = reducer(&snapshot.context, &event, &mut enqueue)?;
        allowed |= !enqueue.events.is_empty() || !enqueue.effects.is_empty();

        if let Update::Replace(context) = update {
            allowed = true;
            if !Rc::ptr_eq(&snapshot.context, &context) {
                snapshot = Rc::new(Snapshot::new(context));
            }
        }

        pending.extend(enqueue.events);
        effects.extend(enqueue.effects);
    }

    Ok(TransitionResult {
        snapshot,
        effects,
        allowed,
    })
}
