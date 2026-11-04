#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlAction {
    Reset,
    Undo,
    Redo,
}
