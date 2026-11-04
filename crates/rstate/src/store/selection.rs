use std::{cell::RefCell, rc::Rc};

use crate::{Selection, Snapshot};

use super::Store;

type Cache<C, T> = Option<(Rc<Snapshot<C>>, Rc<T>)>;

impl<C: 'static, E: 'static, M: 'static> Store<C, E, M> {
    pub fn select<T: 'static>(
        &self,
        selector: impl Fn(&C) -> T + 'static,
        compare: impl Fn(&T, &T) -> bool + 'static,
    ) -> Selection<T> {
        let cache: RefCell<Cache<C, T>> = RefCell::new(None);
        let weak = Rc::downgrade(&self.inner);
        let source = self.inner.clone();
        let observers = self.inner.observers.clone();

        Selection::new(
            move || {
                let inner = weak.upgrade().expect("selection source is alive");
                let snapshot = inner.current.borrow().clone();
                let previous = cache.borrow().clone();
                if let Some((previous_snapshot, value)) = &previous {
                    if Rc::ptr_eq(previous_snapshot, &snapshot) {
                        return value.clone();
                    }
                }

                let next = selector(&snapshot.context);
                let value = match previous {
                    Some((_, previous)) if compare(&previous, &next) => previous,
                    _ => Rc::new(next),
                };
                cache.replace(Some((snapshot, value.clone())));
                value
            },
            move |group| observers.attach(group),
            source,
        )
    }
}
