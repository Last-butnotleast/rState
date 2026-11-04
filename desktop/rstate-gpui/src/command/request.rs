use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use gpui_kit::{AsyncApp, Task};

use super::{CommandPolicy, runner::Runner};

pub type Start = Box<dyn FnOnce(AsyncApp, Weak<Runner>, Weak<Request>) -> Option<Task<()>>>;

pub struct Lane {
    pub policy: CommandPolicy,
    pub running: Cell<Option<u64>>,
    pub latest: Cell<Option<u64>>,
    pub minimum: Cell<u64>,
}

impl Lane {
    pub const fn new(policy: CommandPolicy) -> Self {
        Self {
            policy,
            running: Cell::new(None),
            latest: Cell::new(None),
            minimum: Cell::new(0),
        }
    }

    pub fn release(&self, id: u64) {
        if self.running.get() == Some(id) {
            self.running.set(None);
        }
    }
}

pub struct Request {
    pub id: u64,
    pub active: Cell<bool>,
    pub lane: Rc<Lane>,
    pub start: RefCell<Option<Start>>,
    pub task: RefCell<Option<Task<()>>>,
}

impl Request {
    pub fn new(id: u64, lane: Rc<Lane>, start: Start) -> Self {
        Self {
            id,
            active: Cell::new(true),
            lane,
            start: RefCell::new(Some(start)),
            task: RefCell::new(None),
        }
    }

    pub fn invalidate(&self) {
        self.active.set(false);
        self.lane.release(self.id);
    }

    pub fn cancel(&self) {
        self.invalidate();
        let start = self.start.take();
        let task = self.task.take();
        drop(start);
        drop(task);
    }
}
