use std::rc::Rc;

#[derive(Debug)]
pub struct Snapshot<C> {
    pub context: Rc<C>,
}

impl<C> Snapshot<C> {
    pub const fn new(context: Rc<C>) -> Self {
        Self { context }
    }
}
