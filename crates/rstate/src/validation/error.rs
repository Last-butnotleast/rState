use std::{error::Error, fmt};

use crate::{SchemaMetadata, ValidationIssue, ValidationReason, ValidationStage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub stage: ValidationStage,
    pub reason: ValidationReason,
    pub schema: SchemaMetadata,
    pub name: Option<String>,
    pub issues: Vec<ValidationIssue>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self.reason {
            ValidationReason::InvalidContext => "Invalid context",
            ValidationReason::InvalidEvent => "Invalid event",
            ValidationReason::InvalidEmitted => "Invalid emitted event",
            ValidationReason::UnknownEvent => "Unknown event",
            ValidationReason::UnknownEmitted => "Unknown emitted event",
        };
        f.write_str(description)?;
        if self.reason != ValidationReason::InvalidContext {
            if let Some(name) = &self.name {
                write!(f, " \"{name}\"")?;
            }
        }
        Ok(())
    }
}

impl Error for ValidationError {}
