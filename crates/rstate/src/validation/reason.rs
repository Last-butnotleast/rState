#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationReason {
    InvalidContext,
    InvalidEvent,
    InvalidEmitted,
    UnknownEvent,
    UnknownEmitted,
}
