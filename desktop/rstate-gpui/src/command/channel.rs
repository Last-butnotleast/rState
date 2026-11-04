use std::{
    fmt,
    future::Future,
    rc::{Rc, Weak},
};

use gpui_kit::{App, AsyncApp};
use rstate::{StoreError, StoreResult};

use super::{
    CommandPolicy, CommandSubmitError, CommandTicket,
    request::{Lane, Start},
    runner::Runner,
};

#[derive(Clone)]
pub struct CommandChannel {
    pub(super) owner: Weak<Runner>,
    pub(super) lane: Rc<Lane>,
}

impl fmt::Debug for CommandChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandChannel")
            .field("policy", &self.lane.policy)
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl CommandChannel {
    pub fn policy(&self) -> CommandPolicy {
        self.lane.policy
    }

    pub fn is_closed(&self) -> bool {
        self.owner.upgrade().is_none_or(|owner| owner.is_closed())
    }

    pub fn invalidate(&self) -> usize {
        self.owner
            .upgrade()
            .map_or(0, |owner| owner.invalidate_lane(&self.lane))
    }

    pub fn submit<T: 'static, F: Future<Output = T> + 'static>(
        &self,
        work: impl FnOnce(AsyncApp) -> F + 'static,
        deliver: impl FnOnce(T, &mut App) -> StoreResult<()> + 'static,
        on_error: impl FnOnce(StoreError, &mut App) + 'static,
    ) -> Result<CommandTicket, CommandSubmitError> {
        let owner = self.owner.upgrade().ok_or(CommandSubmitError::Closed)?;
        let start: Start = Box::new(move |cx, owner, request| {
            let future = work(cx.clone());
            if !Runner::accepts(&owner, &request) {
                return None;
            }
            Some(cx.spawn(async move |cx| {
                let output = future.await;
                Runner::complete(&owner, &request, output, deliver, on_error, cx);
            }))
        });
        owner.submit(&self.lane, start)
    }

    pub fn submit_background<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        work: impl FnOnce() -> F + Send + 'static,
        deliver: impl FnOnce(T, &mut App) -> StoreResult<()> + 'static,
        on_error: impl FnOnce(StoreError, &mut App) + 'static,
    ) -> Result<CommandTicket, CommandSubmitError> {
        self.submit(
            move |cx| cx.background_executor().spawn(async move { work().await }),
            deliver,
            on_error,
        )
    }
}
