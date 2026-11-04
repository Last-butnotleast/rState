use std::{fmt, rc::Rc};

use gpui_kit::Context;
use rstate::{ScopedSubscription, Selection};

use crate::notification::NotificationTarget;

mod registration;

pub use registration::Registration;

pub struct ViewBindings<V> {
    pub(crate) target: Rc<NotificationTarget<V>>,
}

impl<V> Clone for ViewBindings<V> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
        }
    }
}

impl<V> fmt::Debug for ViewBindings<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewBindings").finish_non_exhaustive()
    }
}

impl<V: 'static> ViewBindings<V> {
    pub fn new(cx: &Context<V>) -> Self {
        Self {
            target: Rc::new(NotificationTarget::new(cx)),
        }
    }

    pub fn bind<T: 'static>(&self, selection: Selection<T>) -> SelectionBinding<T> {
        let registration = Rc::new(Registration::new());
        let active = registration.clone();
        let target = self.target.clone();
        let subscription = selection
            .subscribe(move |_| {
                target.invalidate(&active);
                Ok(())
            })
            .scoped();
        SelectionBinding {
            selection,
            subscription,
            registration,
        }
    }
}

pub struct SelectionBinding<T> {
    selection: Selection<T>,
    subscription: ScopedSubscription,
    registration: Rc<Registration>,
}

impl<T> fmt::Debug for SelectionBinding<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectionBinding")
            .field("active", &self.registration.is_active())
            .finish_non_exhaustive()
    }
}

impl<T: 'static> SelectionBinding<T> {
    pub fn get(&self) -> Rc<T> {
        self.selection.get()
    }
}

impl<T> Drop for SelectionBinding<T> {
    fn drop(&mut self) {
        self.registration.invalidate();
        self.subscription.unsubscribe();
    }
}
