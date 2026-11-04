use std::fmt;

use super::Subscription;

#[must_use = "keep the guard alive to keep the subscription active"]
pub struct ScopedSubscription {
    subscription: Subscription,
}

impl fmt::Debug for ScopedSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedSubscription")
            .field("subscription", &self.subscription)
            .finish()
    }
}

impl ScopedSubscription {
    pub(super) const fn new(subscription: Subscription) -> Self {
        Self { subscription }
    }

    pub fn unsubscribe(&self) {
        self.subscription.unsubscribe();
    }
}

impl Drop for ScopedSubscription {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}
