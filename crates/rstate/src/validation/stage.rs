#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationStage {
    InitialContext,
    Event,
    Context,
    Emitted,
}
