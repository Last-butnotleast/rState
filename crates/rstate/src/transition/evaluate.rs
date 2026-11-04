use std::rc::Rc;

use crate::{
    Effect, Schemas, Snapshot, StoreResult, TransitionResult, ValidationOptions, ValidationStage,
};

use super::{Reducer, reduce};

pub fn evaluate<C, E, M>(
    reducer: &Reducer<C, E, M>,
    snapshot: &Rc<Snapshot<C>>,
    event: &E,
    schemas: &Schemas<C, E, M>,
    validation: Option<ValidationOptions>,
) -> StoreResult<TransitionResult<C, E, M>> {
    if let Some(options) = validation {
        options.check_event(schemas.events(), event)?;
    }
    let result = reduce(reducer, snapshot.clone(), event)?;
    if let Some(options) = validation {
        options.check_context(
            schemas.context(),
            &result.snapshot.context,
            ValidationStage::Context,
        )?;
        for effect in &result.effects {
            if let Effect::Emit(message) = effect {
                options.check_emitted(schemas.emitted(), message.as_ref())?;
            }
        }
    }
    Ok(result)
}
