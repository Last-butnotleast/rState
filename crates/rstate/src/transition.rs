mod config;
mod reduce;
mod result;
mod update;

pub use config::StoreConfig;
pub use result::TransitionResult;
pub use update::Update;

pub use config::Reducer;
pub use reduce::reduce;
