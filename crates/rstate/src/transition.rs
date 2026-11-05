mod commands;
mod config;
mod evaluate;
mod logic;
mod outcome;
mod reduce;
mod replay;
mod result;
mod update;

pub use commands::CommandResult;
pub use config::StoreConfig;
pub use logic::Transition;
pub use outcome::TransitionOutcome;
pub use replay::{ReplayPolicy, ReplayResult, replay, replay_with_guard};
pub use result::TransitionResult;
pub use update::Update;

pub use config::Reducer;
pub use evaluate::evaluate;
use reduce::reduce;
