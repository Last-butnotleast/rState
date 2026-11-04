use std::rc::Rc;

use crate::{ControlAction, Snapshot};

#[derive(Debug)]
#[non_exhaustive]
pub enum Inspection<'a, C, E> {
    #[non_exhaustive]
    Initial { snapshot: Rc<Snapshot<C>> },
    #[non_exhaustive]
    Transition {
        event: &'a E,
        snapshot: Rc<Snapshot<C>>,
    },
    #[non_exhaustive]
    Control {
        action: ControlAction,
        snapshot: Rc<Snapshot<C>>,
    },
}
