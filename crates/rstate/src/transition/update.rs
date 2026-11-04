use std::rc::Rc;

#[derive(Debug)]
pub enum Update<C> {
    Keep,
    Replace(Rc<C>),
}

impl<C> Update<C> {
    pub fn replace(context: C) -> Self {
        Self::Replace(Rc::new(context))
    }
}
