use std::{cell::Cell, rc::Rc};

pub struct Subscription {
    inner: Rc<SubscriptionInner>,
}

struct SubscriptionInner {
    active: Cell<bool>,
    cancel: Box<dyn Fn()>,
}

impl Clone for Subscription {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Subscription {
    pub(crate) fn new(cancel: impl Fn() + 'static) -> Self {
        Self {
            inner: Rc::new(SubscriptionInner {
                active: Cell::new(true),
                cancel: Box::new(cancel),
            }),
        }
    }

    pub fn unsubscribe(&self) {
        if self.inner.active.replace(false) {
            (self.inner.cancel)();
        }
    }
}
