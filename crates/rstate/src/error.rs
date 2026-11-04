use std::error::Error;

use crate::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0}")]
    Callback(String),
    #[error("{0}")]
    External(#[source] Box<dyn Error + Send + Sync>),
    #[error("{0}")]
    Validation(
        #[from]
        #[source]
        ValidationError,
    ),
}

impl StoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Callback(message.into())
    }

    pub fn from_error(error: impl Error + Send + Sync + 'static) -> Self {
        Self::External(Box::new(error))
    }

    pub fn validation_error(&self) -> Option<&ValidationError> {
        match self {
            Self::Validation(error) => Some(error),
            Self::External(error) => error.downcast_ref(),
            Self::Callback(_) => None,
        }
    }
}

pub type StoreResult<T = ()> = Result<T, StoreError>;
