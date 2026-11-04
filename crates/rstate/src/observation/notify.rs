use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

use crate::{StoreResult, Subscription};

mod slot;

pub use slot::ObserverSlot;

thread_local! {
    static PENDING: RefCell<VecDeque<Rc<Notification>>> = const { RefCell::new(VecDeque::new()) };
}

struct Notification {
    active: Cell<bool>,
    queued: Cell<bool>,
    running: Cell<bool>,
    callback: Box<dyn Fn() -> StoreResult<()>>,
}

struct Running<'a>(&'a Cell<bool>);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl Notification {
    fn schedule(self: &Rc<Self>) {
        if self.active.get() && !self.running.get() && !self.queued.replace(true) {
            PENDING.with_borrow_mut(|pending| pending.push_back(self.clone()));
        }
    }

    fn run(&self) -> StoreResult<()> {
        self.queued.set(false);
        if !self.active.get() {
            return Ok(());
        }
        self.running.set(true);
        let _running = Running(&self.running);
        (self.callback)()
    }
}

enum Node {
    Task(Rc<Notification>),
    Group(Observers),
}

impl Clone for Node {
    fn clone(&self) -> Self {
        match self {
            Self::Task(task) => Self::Task(task.clone()),
            Self::Group(group) => Self::Group(group.clone()),
        }
    }
}

impl Node {
    fn schedule(&self) {
        match self {
            Self::Task(task) => task.schedule(),
            Self::Group(group) => group.schedule(),
        }
    }

    fn cancel(&self) {
        if let Self::Task(task) = self {
            task.active.set(false);
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct Observers {
    inner: Rc<ObserverEntries>,
}

#[derive(Default)]
struct ObserverEntries {
    next_id: Cell<u64>,
    dependencies: Cell<u64>,
    entries: RefCell<BTreeMap<u64, Node>>,
}

impl ObserverEntries {
    fn reserve(&self) -> u64 {
        let id = self.next_id.get();
        #[expect(
            clippy::expect_used,
            reason = "exhausting monotonic subscription IDs cannot preserve registration order"
        )]
        self.next_id.set(
            id.checked_add(1)
                .expect("observer subscription IDs exhausted"),
        );
        id
    }
}

impl Observers {
    fn insert(&self, node: Node) -> Subscription {
        let id = self.inner.reserve();
        self.inner.entries.borrow_mut().insert(id, node);
        let weak = Rc::downgrade(&self.inner);

        Subscription::new(move || {
            if let Some(inner) = weak.upgrade() {
                let node = inner.entries.borrow_mut().remove(&id);
                if let Some(node) = node {
                    node.cancel();
                }
            }
        })
    }

    pub(crate) fn subscribe(
        &self,
        callback: impl Fn() -> StoreResult<()> + 'static,
    ) -> Subscription {
        self.insert(Node::Task(Rc::new(Notification {
            active: Cell::new(true),
            queued: Cell::new(false),
            running: Cell::new(false),
            callback: Box::new(callback),
        })))
    }

    pub(crate) fn slot(&self) -> ObserverSlot {
        ObserverSlot::new(Rc::downgrade(&self.inner))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.entries.borrow().is_empty()
    }

    pub(crate) fn has_dependencies(&self) -> bool {
        self.inner.dependencies.get() != 0
    }

    fn schedule(&self) {
        let nodes: Vec<_> = self.inner.entries.borrow().values().cloned().collect();
        for node in nodes {
            node.schedule();
        }
    }

    pub(crate) fn publish(&self) -> StoreResult<()> {
        self.schedule();
        flush()
    }
}

fn flush() -> StoreResult<()> {
    let mut first_error = None;
    loop {
        let next = PENDING.with_borrow_mut(VecDeque::pop_front);
        let Some(notification) = next else {
            break;
        };
        if let Err(error) = notification.run() {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}
