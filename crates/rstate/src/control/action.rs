#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    Reset,
    Undo,
    Redo,
}
