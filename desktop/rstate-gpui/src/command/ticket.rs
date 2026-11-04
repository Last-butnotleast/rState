use std::{
    fmt,
    rc::{Rc, Weak},
};

use super::{request::Request, runner::Runner};

#[derive(Clone)]
pub struct CommandTicket {
    owner: Weak<Runner>,
    request: Weak<Request>,
    id: u64,
}

impl fmt::Debug for CommandTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandTicket")
            .field("id", &self.id)
            .field("pending", &self.is_pending())
            .finish_non_exhaustive()
    }
}

impl CommandTicket {
    pub(super) fn new(owner: &Rc<Runner>, request: &Rc<Request>) -> Self {
        Self {
            owner: Rc::downgrade(owner),
            request: Rc::downgrade(request),
            id: request.id,
        }
    }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn is_pending(&self) -> bool {
        self.request
            .upgrade()
            .is_some_and(|request| request.active.get())
    }

    pub fn cancel(&self) -> bool {
        self.owner
            .upgrade()
            .is_some_and(|owner| owner.cancel(self.id))
    }
}
