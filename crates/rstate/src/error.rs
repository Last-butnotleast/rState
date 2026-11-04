use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0}")]
    Callback(String),
    #[error("{0}")]
    External(#[source] Box<dyn Error + Send + Sync>),
}

impl StoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Callback(message.into())
    }

    pub fn from_error(error: impl Error + Send + Sync + 'static) -> Self {
        Self::External(Box::new(error))
    }
}

pub type StoreResult<T = ()> = Result<T, StoreError>;
