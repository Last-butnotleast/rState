use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use crate::{StoreResult, Subscription};

use super::{ObserverSlot, Observers};

mod map;

type Getter<T> = dyn Fn() -> Rc<T>;
type Attach = dyn Fn(Subscription) -> Subscription;

pub struct Selection<T> {
    get: Rc<Getter<T>>,
    observers: Observers,
    attach: Rc<Attach>,
    upstream: Rc<RefCell<Option<Subscription>>>,
    position: ObserverSlot,
    invalidated: Rc<Cell<bool>>,
    source: Rc<dyn Any>,
}

impl<T> fmt::Debug for Selection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let observed = self
            .upstream
            .try_borrow()
            .map(|upstream| upstream.is_some())
            .ok();
        let mut debug = f.debug_struct("Selection");
        match observed {
            Some(observed) => debug.field("observed", &observed),
            None => debug.field("observed", &"<borrowed>"),
        };
        debug.finish_non_exhaustive()
    }
}

impl<T> Clone for Selection<T> {
    fn clone(&self) -> Self {
        Self {
            get: self.get.clone(),
            observers: self.observers.clone(),
            attach: self.attach.clone(),
            upstream: self.upstream.clone(),
            position: self.position.clone(),
            invalidated: self.invalidated.clone(),
            source: self.source.clone(),
        }
    }
}

impl<T: 'static> Selection<T> {
    pub(crate) fn new(
        get: impl Fn(bool) -> Rc<T> + 'static,
        attach: impl Fn(Subscription) -> Subscription + 'static,
        source: Rc<dyn Any>,
        position: ObserverSlot,
    ) -> Self {
        let invalidated = Rc::new(Cell::new(false));
        let refresh = invalidated.clone();
        let dependency = position.clone();
        Self {
            get: Rc::new(move || {
                let value = get(refresh.replace(false));
                dependency.reserve();
                value
            }),
            observers: Observers::default(),
            attach: Rc::new(attach),
            upstream: Rc::new(RefCell::new(None)),
            position,
            invalidated,
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

        self.observe(local)
    }

    fn observe(&self, local: Subscription) -> Subscription {
        if self.upstream.borrow().is_none() {
            let registration = self.position.attach(self.observers.clone());
            let upstream = (self.attach)(registration);
            self.upstream.replace(Some(upstream));
        }

        let observers = self.observers.clone();
        let upstream = self.upstream.clone();
        let invalidated = self.invalidated.clone();
        let position = self.position.clone();
        Subscription::new(move || {
            local.unsubscribe();
            if observers.is_empty() {
                if !observers.has_dependencies() {
                    invalidated.set(true);
                    position.clear();
                }
                let source = upstream.borrow_mut().take();
                if let Some(source) = source {
                    source.unsubscribe();
                }
            }
        })
    }
}
