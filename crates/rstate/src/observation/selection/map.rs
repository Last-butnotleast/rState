use std::{cell::RefCell, rc::Rc};

use super::Selection;

type Cache<T, U> = Option<(Rc<T>, Rc<U>)>;

impl<T: 'static> Selection<T> {
    pub fn map<U: 'static>(
        &self,
        project: impl Fn(&T) -> U + 'static,
        compare: impl Fn(&U, &U) -> bool + 'static,
    ) -> Selection<U> {
        let cache: RefCell<Cache<T, U>> = RefCell::new(None);
        let get = self.get.clone();
        let upstream = self.clone();

        Selection::new(
            move |refresh| {
                let source = get();
                let previous = cache.borrow().clone();
                if let Some((previous_source, value)) = &previous {
                    if !refresh && Rc::ptr_eq(previous_source, &source) {
                        return value.clone();
                    }
                }

                let next = project(&source);
                let value = match previous {
                    Some((_, previous)) if compare(&previous, &next) => previous,
                    _ => Rc::new(next),
                };
                cache.replace(Some((source, value.clone())));
                value
            },
            move |registration| upstream.observe(registration),
            self.source.clone(),
            self.observers.slot(),
        )
    }
}
