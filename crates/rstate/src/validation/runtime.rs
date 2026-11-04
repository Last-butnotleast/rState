use crate::{
    Schema, SchemaCheck, StoreResult, UnknownPolicy, ValidationError, ValidationOptions,
    ValidationReason, ValidationStage,
};

impl ValidationOptions {
    pub(crate) fn check_context<T>(
        self,
        schema: Option<&Schema<T>>,
        value: &T,
        stage: ValidationStage,
    ) -> StoreResult<()> {
        if self.context {
            validate(schema, value, stage, UnknownPolicy::Reject)?;
        }
        Ok(())
    }

    pub(crate) fn check_event<T>(self, schema: Option<&Schema<T>>, value: &T) -> StoreResult<()> {
        if self.events {
            validate(schema, value, ValidationStage::Event, self.unknown_events)?;
        }
        Ok(())
    }

    pub(crate) fn check_emitted<T>(self, schema: Option<&Schema<T>>, value: &T) -> StoreResult<()> {
        if self.emitted {
            validate(
                schema,
                value,
                ValidationStage::Emitted,
                self.unknown_emitted,
            )?;
        }
        Ok(())
    }
}

fn validate<T>(
    schema: Option<&Schema<T>>,
    value: &T,
    stage: ValidationStage,
    unknown: UnknownPolicy,
) -> StoreResult<()> {
    let Some(schema) = schema else {
        return Ok(());
    };
    let (reason, name, issues) = match schema.check(value)? {
        SchemaCheck::Valid => return Ok(()),
        SchemaCheck::Invalid { issues, name } => {
            let reason = match stage {
                ValidationStage::InitialContext | ValidationStage::Context => {
                    ValidationReason::InvalidContext
                }
                ValidationStage::Event => ValidationReason::InvalidEvent,
                ValidationStage::Emitted => ValidationReason::InvalidEmitted,
            };
            (reason, name, issues)
        }
        SchemaCheck::Unknown { name } => {
            if unknown == UnknownPolicy::Ignore {
                return Ok(());
            }
            let reason = match stage {
                ValidationStage::InitialContext | ValidationStage::Context => {
                    ValidationReason::InvalidContext
                }
                ValidationStage::Event => ValidationReason::UnknownEvent,
                ValidationStage::Emitted => ValidationReason::UnknownEmitted,
            };
            (reason, Some(name), Vec::new())
        }
    };
    Err(ValidationError {
        stage,
        reason,
        schema: schema.metadata().clone(),
        name,
        issues,
    }
    .into())
}
