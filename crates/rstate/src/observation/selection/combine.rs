use std::{cell::RefCell, rc::Rc};

use crate::Subscription;

use super::Selection;

type Cache<T, R, U> = Option<(Rc<T>, Rc<R>, Rc<U>)>;

impl<T: 'static> Selection<T> {
    pub fn combine<R: 'static, U: 'static>(
        &self,
        right: &Selection<R>,
        project: impl Fn(&T, &R) -> U + 'static,
        compare: impl Fn(&U, &U) -> bool + 'static,
    ) -> Selection<U> {
        let cache: RefCell<Cache<T, R, U>> = RefCell::new(None);
        let left_get = self.get.clone();
        let right_get = right.get.clone();
        let left = self.clone();
        let right = right.clone();
        let left_position = self.observers.slot();
        let right_position =
            (!self.observers.is_same(&right.observers)).then(|| right.observers.slot());
        let mut positions = vec![left_position.clone()];
        positions.extend(right_position.iter().cloned());
        let left_read = left_position.clone();
        let right_read = right_position.clone();
        let source = Rc::new((self.source.clone(), right.source.clone()));

        Selection::with_dependencies(
            move |refresh| {
                let left = left_get();
                left_read.reserve();
                let right = right_get();
                if let Some(position) = &right_read {
                    position.reserve();
                }
                let previous = cache.borrow().clone();
                if let Some((previous_left, previous_right, value)) = &previous {
                    if !refresh
                        && Rc::ptr_eq(previous_left, &left)
                        && Rc::ptr_eq(previous_right, &right)
                    {
                        return value.clone();
                    }
                }

                let next = project(&left, &right);
                let value = match previous {
                    Some((_, _, previous)) if compare(&previous, &next) => previous,
                    _ => Rc::new(next),
                };
                cache.replace(Some((left, right, value.clone())));
                value
            },
            move |observers| {
                let first = left.observe(left_position.attach(observers.clone()));
                let second = right_position
                    .as_ref()
                    .map(|position| right.observe(position.attach(observers)));
                Subscription::new(move || {
                    first.unsubscribe();
                    if let Some(second) = &second {
                        second.unsubscribe();
                    }
                })
            },
            source,
            positions,
        )
    }
}
