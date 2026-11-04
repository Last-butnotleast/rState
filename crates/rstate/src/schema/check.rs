use crate::ValidationIssue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaCheck {
    Valid,
    Invalid {
        issues: Vec<ValidationIssue>,
        name: Option<String>,
    },
    Unknown {
        name: String,
    },
}

impl SchemaCheck {
    pub const fn invalid(issues: Vec<ValidationIssue>) -> Self {
        Self::Invalid { issues, name: None }
    }

    pub fn unknown(name: impl Into<String>) -> Self {
        Self::Unknown { name: name.into() }
    }

    #[must_use]
    pub fn named(self, name: impl Into<String>) -> Self {
        match self {
            Self::Valid => Self::Valid,
            Self::Invalid { issues, .. } => Self::Invalid {
                issues,
                name: Some(name.into()),
            },
            Self::Unknown { .. } => Self::Unknown { name: name.into() },
        }
    }
}
