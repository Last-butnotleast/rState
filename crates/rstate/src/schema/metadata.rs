#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SchemaMetadata {
    pub name: String,
    pub vendor: Option<String>,
}

impl SchemaMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vendor: None,
        }
    }

    #[must_use]
    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = Some(vendor.into());
        self
    }
}
