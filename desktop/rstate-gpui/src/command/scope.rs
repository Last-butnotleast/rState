use std::{fmt, rc::Rc};

use gpui::App;

use super::{CommandChannel, CommandPolicy, request::Lane, runner::Runner};

pub struct CommandScope {
    inner: Rc<Runner>,
}

impl fmt::Debug for CommandScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandScope")
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl CommandScope {
    pub fn new(cx: &App) -> Self {
        Self {
            inner: Rc::new(Runner::new(cx)),
        }
    }

    pub fn channel(&self, policy: CommandPolicy) -> CommandChannel {
        CommandChannel {
            owner: Rc::downgrade(&self.inner),
            lane: Rc::new(Lane::new(policy)),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    pub fn invalidate(&self) -> usize {
        self.inner.invalidate()
    }

    pub fn close(&self) -> usize {
        self.inner.close()
    }
}

impl Drop for CommandScope {
    fn drop(&mut self) {
        self.close();
    }
}
