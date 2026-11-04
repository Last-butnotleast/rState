#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HistoryStatus {
    pub past: usize,
    pub future: usize,
    pub limit: Option<usize>,
}
