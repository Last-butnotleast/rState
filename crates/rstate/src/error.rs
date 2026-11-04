use std::{error::Error, fmt};

use crate::ValidationError;

mod status;

pub use status::DispatchStatus;

#[derive(Debug)]
pub struct StoreError {
    cause: Cause,
    dispatch_status: Option<DispatchStatus>,
}

#[derive(Debug, thiserror::Error)]
enum Cause {
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
        Self::from_cause(Cause::Callback(message.into()))
    }

    pub fn from_error(error: impl Error + Send + Sync + 'static) -> Self {
        Self::from_cause(Cause::External(Box::new(error)))
    }

    pub fn validation_error(&self) -> Option<&ValidationError> {
        match &self.cause {
            Cause::Validation(error) => Some(error),
            Cause::External(error) => error.downcast_ref(),
            Cause::Callback(_) => None,
        }
    }

    pub const fn dispatch_status(&self) -> Option<DispatchStatus> {
        self.dispatch_status
    }

    pub(crate) const fn with_dispatch_status(mut self, status: DispatchStatus) -> Self {
        self.dispatch_status = Some(status);
        self
    }

    pub(crate) const fn without_dispatch_status(mut self) -> Self {
        self.dispatch_status = None;
        self
    }

    const fn from_cause(cause: Cause) -> Self {
        Self {
            cause,
            dispatch_status: None,
        }
    }
}

impl From<ValidationError> for StoreError {
    fn from(error: ValidationError) -> Self {
        Self::from_cause(Cause::Validation(error))
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.cause.fmt(f)
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.source()
    }
}

pub type StoreResult<T = ()> = Result<T, StoreError>;
