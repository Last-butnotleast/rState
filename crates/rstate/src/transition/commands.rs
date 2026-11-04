use std::{fmt, rc::Rc};

use crate::{Effect, Snapshot, TransitionResult};

#[non_exhaustive]
pub struct CommandResult<C, M> {
    pub snapshot: Rc<Snapshot<C>>,
    pub commands: Vec<Rc<M>>,
    pub allowed: bool,
}

impl<C: fmt::Debug, M> fmt::Debug for CommandResult<C, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandResult")
            .field("snapshot", &self.snapshot)
            .field("command_count", &self.commands.len())
            .field("allowed", &self.allowed)
            .finish()
    }
}

impl<C, E, M> TransitionResult<C, E, M> {
    pub fn into_commands(self) -> Result<CommandResult<C, M>, Self> {
        if self
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Run(_)))
        {
            return Err(self);
        }
        let commands = self
            .effects
            .into_iter()
            .filter_map(|effect| match effect {
                Effect::Emit(message) => Some(message),
                Effect::Run(_) => None,
            })
            .collect();
        Ok(CommandResult {
            snapshot: self.snapshot,
            commands,
            allowed: self.allowed,
        })
    }
}
