use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use gpui::{AsyncApp, Context, WeakEntity};

use crate::binding::Registration;

pub struct NotificationTarget<V> {
    entity: WeakEntity<V>,
    app: AsyncApp,
    pending: Cell<bool>,
    dirty: RefCell<Vec<Weak<Registration>>>,
}

impl<V: 'static> NotificationTarget<V> {
    pub(crate) fn new(cx: &Context<V>) -> Self {
        Self {
            entity: cx.entity().downgrade(),
            app: cx.to_async(),
            pending: Cell::new(false),
            dirty: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn invalidate(self: &Rc<Self>, registration: &Rc<Registration>) {
        if !registration.is_active() || !registration.mark_dirty() {
            return;
        }
        self.dirty.borrow_mut().push(Rc::downgrade(registration));
        if self.pending.replace(true) {
            return;
        }
        let target = self.clone();
        self.app.spawn(async move |cx| target.flush(cx)).detach();
    }

    fn flush(&self, cx: &mut AsyncApp) {
        self.pending.set(false);
        let dirty = self.dirty.take();
        let mut notify = false;
        for registration in dirty.into_iter().filter_map(|value| value.upgrade()) {
            registration.clear_dirty();
            notify |= registration.is_active();
        }
        if notify {
            let _ = self.entity.update(cx, |_, cx| cx.notify());
        }
    }
}
