use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0}")]
    Callback(String),
    #[error(transparent)]
    External(Box<dyn Error>),
}

impl StoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Callback(message.into())
    }

    pub fn from_error(error: impl Error + 'static) -> Self {
        Self::External(Box::new(error))
    }
}

pub type StoreResult<T = ()> = Result<T, StoreError>;
