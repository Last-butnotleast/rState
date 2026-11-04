use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
};

use gpui_kit::{App, AsyncApp, Subscription};
use rstate::{StoreError, StoreResult};

use super::{
    CommandPolicy, CommandSubmitError, CommandTicket,
    request::{Lane, Request, Start},
};

#[derive(Default)]
struct State {
    requests: Vec<Rc<Request>>,
    queued: VecDeque<Rc<Request>>,
}

pub struct Runner {
    app: AsyncApp,
    closed: Cell<bool>,
    next_id: Cell<u64>,
    minimum: Cell<u64>,
    scheduled: Cell<bool>,
    state: RefCell<State>,
    _quit: Subscription,
}

impl Runner {
    pub fn new(cx: &App) -> Rc<Self> {
        Rc::new_cyclic(|owner: &Weak<Self>| {
            let owner = owner.clone();
            let quit = cx.on_app_quit(move |_| {
                if let Some(owner) = owner.upgrade() {
                    owner.close();
                }
                async {}
            });
            Self {
                app: cx.to_async(),
                closed: Cell::new(false),
                next_id: Cell::new(1),
                minimum: Cell::new(0),
                scheduled: Cell::new(false),
                state: RefCell::new(State::default()),
                _quit: quit,
            }
        })
    }

    pub const fn is_closed(&self) -> bool {
        self.closed.get()
    }

    pub fn submit(
        self: &Rc<Self>,
        lane: &Rc<Lane>,
        start: Start,
    ) -> Result<CommandTicket, CommandSubmitError> {
        if self.is_closed() {
            return Err(CommandSubmitError::Closed);
        }
        let id = self.next_id.get();
        let next = id
            .checked_add(1)
            .ok_or(CommandSubmitError::IdentityExhausted)?;
        self.next_id.set(next);
        let request = Rc::new(Request::new(id, lane.clone(), start));
        let ticket = CommandTicket::new(self, &request);
        if lane.policy == CommandPolicy::Latest {
            lane.latest.set(Some(id));
            self.cancel_where(|request| Rc::ptr_eq(&request.lane, lane));
        }
        if self.is_closed() {
            request.cancel();
            return Err(CommandSubmitError::Closed);
        }
        if id < self.minimum.get()
            || id < lane.minimum.get()
            || (lane.policy == CommandPolicy::Latest && lane.latest.get() != Some(id))
        {
            request.cancel();
            return Ok(ticket);
        }
        {
            let mut state = self.state.borrow_mut();
            state.requests.push(request.clone());
            state.queued.push_back(request);
        }
        self.schedule();
        Ok(ticket)
    }

    pub fn cancel(self: &Rc<Self>, id: u64) -> bool {
        let count = self.cancel_where(|request| request.id == id);
        self.schedule();
        count != 0
    }

    pub fn invalidate_lane(self: &Rc<Self>, lane: &Rc<Lane>) -> usize {
        lane.minimum.set(self.next_id.get());
        lane.latest.set(None);
        let count = self.cancel_where(|request| Rc::ptr_eq(&request.lane, lane));
        self.schedule();
        count
    }

    pub fn invalidate(self: &Rc<Self>) -> usize {
        self.minimum.set(self.next_id.get());
        self.cancel_where(|_| true)
    }

    pub fn close(self: &Rc<Self>) -> usize {
        self.closed.set(true);
        self.invalidate()
    }

    fn cancel_where(&self, matches: impl Fn(&Request) -> bool) -> usize {
        let (cancelled, draining, queued) = {
            let mut state = self.state.borrow_mut();
            let requests = std::mem::take(&mut state.requests);
            let mut cancelled = Vec::new();
            let mut draining = Vec::new();
            let mut retained = Vec::new();
            for request in requests {
                if matches(&request) && (request.active.get() || self.is_closed()) {
                    if !self.is_closed()
                        && request.lane.policy == CommandPolicy::Serial
                        && request.lane.running.get() == Some(request.id)
                    {
                        draining.push(request.clone());
                        retained.push(request);
                    } else {
                        cancelled.push(request);
                    }
                } else {
                    retained.push(request);
                }
            }
            state.requests = retained;
            (cancelled, draining, std::mem::take(&mut state.queued))
        };
        let count = cancelled.len().saturating_add(draining.len());
        for request in &draining {
            request.active.set(false);
        }
        for request in &cancelled {
            request.invalidate();
        }
        let (queued, expired): (VecDeque<_>, VecDeque<_>) =
            queued.into_iter().partition(|request| request.active.get());
        self.state.borrow_mut().queued = queued;
        for request in cancelled {
            request.cancel();
        }
        drop(expired);
        count
    }

    fn schedule(self: &Rc<Self>) {
        if self.is_closed() || self.state.borrow().queued.is_empty() || self.scheduled.replace(true)
        {
            return;
        }
        let owner = Rc::downgrade(self);
        self.app
            .spawn(async move |cx| {
                if let Some(owner) = owner.upgrade() {
                    owner.drain(cx);
                }
            })
            .detach();
    }

    fn drain(self: &Rc<Self>, cx: &AsyncApp) {
        self.scheduled.set(false);
        let count = self.state.borrow().queued.len();
        for _ in 0..count {
            let request = self.state.borrow_mut().queued.pop_front();
            let Some(request) = request else {
                break;
            };
            if self.is_closed() || !request.active.get() {
                continue;
            }
            if request.lane.policy == CommandPolicy::Serial {
                if request.lane.running.get().is_some() {
                    self.state.borrow_mut().queued.push_back(request);
                    continue;
                }
                request.lane.running.set(Some(request.id));
            }
            let start = request.start.take();
            if let Some(start) = start {
                let task = start(cx.clone(), Rc::downgrade(self), Rc::downgrade(&request));
                if Self::accepts(&Rc::downgrade(self), &Rc::downgrade(&request)) {
                    request.task.replace(task);
                } else {
                    drop(task);
                    let removed = self.remove(request.id);
                    request.lane.release(request.id);
                    drop(removed);
                    self.schedule();
                }
            }
        }
    }

    fn remove(&self, id: u64) -> Option<Rc<Request>> {
        let mut state = self.state.borrow_mut();
        let position = state.requests.iter().position(|request| request.id == id);
        position.map(|position| state.requests.remove(position))
    }

    pub fn complete<T>(
        owner: &Weak<Self>,
        request: &Weak<Request>,
        output: T,
        deliver: impl FnOnce(T, &mut App) -> StoreResult<()>,
        on_error: impl FnOnce(StoreError, &mut App),
        cx: &AsyncApp,
    ) {
        let Some(owner) = owner.upgrade() else {
            return;
        };
        let Some(request) = request.upgrade() else {
            return;
        };
        let active = request.active.replace(false) && !owner.is_closed();
        let removed = owner.remove(request.id);
        let task = request.task.take();
        if let Some(task) = task {
            task.detach();
        }
        drop(removed);
        if active && let Err(error) = cx.update(|cx| deliver(output, cx)) {
            cx.update(|cx| on_error(error, cx));
        }
        request.lane.release(request.id);
        owner.schedule();
    }

    pub fn accepts(owner: &Weak<Self>, request: &Weak<Request>) -> bool {
        owner.upgrade().is_some_and(|owner| !owner.is_closed())
            && request.upgrade().is_some_and(|request| {
                request.active.get()
                    || (request.lane.policy == CommandPolicy::Serial
                        && request.lane.running.get() == Some(request.id))
            })
    }
}
