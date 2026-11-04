#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HistoryStatus {
    pub past: usize,
    pub future: usize,
    pub limit: Option<usize>,
}
