#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationStage {
    InitialContext,
    Event,
    Context,
    Emitted,
}
