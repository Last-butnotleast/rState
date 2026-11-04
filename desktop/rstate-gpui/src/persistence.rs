use std::{fmt, rc::Rc};

use rstate::{PersistenceStatus, ScopedSubscription, Store, StoreResult};

use crate::ViewBindings;
use crate::binding::Registration;

type Reader = dyn Fn() -> Rc<PersistenceStatus>;

pub struct PersistenceBinding {
    read: Box<Reader>,
    subscription: ScopedSubscription,
    registration: Rc<Registration>,
}

impl fmt::Debug for PersistenceBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistenceBinding")
            .field("active", &self.registration.is_active())
            .finish_non_exhaustive()
    }
}

impl PersistenceBinding {
    pub fn get(&self) -> Rc<PersistenceStatus> {
        (self.read)()
    }
}

impl<V: 'static> ViewBindings<V> {
    pub fn bind_persistence<C: 'static, E: 'static, M: 'static>(
        &self,
        store: &Store<C, E, M>,
    ) -> StoreResult<PersistenceBinding> {
        let registration = Rc::new(Registration::new());
        let active = registration.clone();
        let target = self.target.clone();
        let subscription = store
            .subscribe_persistence(move |_| target.invalidate(&active))?
            .scoped();
        let store = store.clone();
        Ok(PersistenceBinding {
            read: Box::new(move || {
                #[expect(
                    clippy::expect_used,
                    reason = "a persistence binding is constructed only for configured stores"
                )]
                store
                    .persistence_status()
                    .expect("persistence is configured")
            }),
            subscription,
            registration,
        })
    }
}

impl Drop for PersistenceBinding {
    fn drop(&mut self) {
        self.registration.invalidate();
        self.subscription.unsubscribe();
    }
}
