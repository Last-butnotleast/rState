use std::{cell::Cell, fmt, rc::Rc};

mod scoped;

pub use scoped::ScopedSubscription;

pub struct Subscription {
    inner: Rc<SubscriptionInner>,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field("active", &self.inner.active.get())
            .finish_non_exhaustive()
    }
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
    pub const fn scoped(self) -> ScopedSubscription {
        ScopedSubscription::new(self)
    }

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
