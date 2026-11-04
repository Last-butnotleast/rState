use std::rc::Rc;

use crate::Snapshot;

#[derive(Debug)]
pub enum Inspection<'a, C, E> {
    Initial {
        snapshot: Rc<Snapshot<C>>,
    },
    Transition {
        event: &'a E,
        snapshot: Rc<Snapshot<C>>,
    },
}
