mod config;
mod reduce;
mod result;
mod update;

pub use config::StoreConfig;
pub use result::TransitionResult;
pub use update::Update;

pub(crate) use config::Reducer;
pub(crate) use reduce::reduce;
