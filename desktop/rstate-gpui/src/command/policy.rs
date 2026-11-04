#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandPolicy {
    Concurrent,
    Serial,
    Latest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CommandSubmitError {
    #[error("the command scope is closed")]
    Closed,
    #[error("command request identities are exhausted")]
    IdentityExhausted,
}
