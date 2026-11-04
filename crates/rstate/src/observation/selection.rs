use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{StoreResult, Subscription};

use super::Observers;

type Getter<T> = dyn Fn() -> Rc<T>;
type Attach = dyn Fn(Observers) -> Subscription;

pub struct Selection<T> {
    get: Rc<Getter<T>>,
    observers: Observers,
    attach: Rc<Attach>,
    upstream: Rc<RefCell<Option<Subscription>>>,
    source: Rc<dyn Any>,
}

impl<T> Clone for Selection<T> {
    fn clone(&self) -> Self {
        Self {
            get: self.get.clone(),
            observers: self.observers.clone(),
            attach: self.attach.clone(),
            upstream: self.upstream.clone(),
            source: self.source.clone(),
        }
    }
}

impl<T: 'static> Selection<T> {
    pub(crate) fn new(
        get: impl Fn() -> Rc<T> + 'static,
        attach: impl Fn(Observers) -> Subscription + 'static,
        source: Rc<dyn Any>,
    ) -> Self {
        Self {
            get: Rc::new(get),
            observers: Observers::default(),
            attach: Rc::new(attach),
            upstream: Rc::new(RefCell::new(None)),
            source,
        }
    }

    pub fn get(&self) -> Rc<T> {
        (self.get)()
    }

    pub fn subscribe(&self, listener: impl Fn(Rc<T>) -> StoreResult<()> + 'static) -> Subscription {
        let last = RefCell::new(self.get());
        let get = self.get.clone();
        let local = self.observers.subscribe(move || {
            let value = get();
            if Rc::ptr_eq(&last.borrow(), &value) {
                return Ok(());
            }
            last.replace(value.clone());
            listener(value)?;
            last.replace(get());
            Ok(())
        });

        if self.upstream.borrow().is_none() {
            let upstream = (self.attach)(self.observers.clone());
            self.upstream.replace(Some(upstream));
        }

        let observers = self.observers.clone();
        let upstream = self.upstream.clone();
        Subscription::new(move || {
            local.unsubscribe();
            if observers.is_empty() {
                let source = upstream.borrow_mut().take();
                if let Some(source) = source {
                    source.unsubscribe();
                }
            }
        })
    }
}
