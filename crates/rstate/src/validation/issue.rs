#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ValidationIssue {
    pub message: String,
    pub path: Vec<PathSegment>,
}

impl ValidationIssue {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_path(mut self, path: Vec<PathSegment>) -> Self {
        self.path = path;
        self
    }
}
