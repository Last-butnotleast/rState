#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationReason {
    InvalidContext,
    InvalidEvent,
    InvalidEmitted,
    UnknownEvent,
    UnknownEmitted,
}
