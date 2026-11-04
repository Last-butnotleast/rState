mod config;
mod evaluate;
mod logic;
mod reduce;
mod result;
mod update;

pub use config::StoreConfig;
pub use logic::Transition;
pub use result::TransitionResult;
pub use update::Update;

pub use config::Reducer;
pub use evaluate::evaluate;
use reduce::reduce;
