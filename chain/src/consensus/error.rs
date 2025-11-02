use std::fmt;

/// Error type returned when a block fails validation.
#[derive(Debug)]
pub enum ValidationError {
    /// Block is invalid according to a validity predicate.
    Invalid(&'static str),
    /// Block is invalid with a dynamic error message.
    Custom(String),
}

/// High-level errors that can occur in the consensus engine.
#[derive(Debug)]
pub enum ConsensusError {
    /// Underlying validation failure.
    Validation(ValidationError),
    /// Storage-related failure, e.g. missing parent block.
    Storage(String),
    /// Catch-all for other issues.
    Other(String),
}

impl From<ValidationError> for ConsensusError {
    fn from(e: ValidationError) -> Self {
        ConsensusError::Validation(e)
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Invalid(msg) => write!(f, "invalid block: {msg}"),
            ValidationError::Custom(msg) => write!(f, "invalid block: {msg}"),
        }
    }
}

impl fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusError::Validation(e) => write!(f, "{e}"),
            ConsensusError::Storage(msg) => write!(f, "storage error: {msg}"),
            ConsensusError::Other(msg) => write!(f, "consensus error: {msg}"),
        }
    }
}

impl std::error::Error for ValidationError {}
impl std::error::Error for ConsensusError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn validation_error_display_static_invalid() {
        let err = ValidationError::Invalid("bad nonce");
        assert_eq!(err.to_string(), "invalid block: bad nonce");
    }

    #[test]
    fn validation_error_display_custom() {
        let err = ValidationError::Custom("height mismatch".to_string());
        assert_eq!(err.to_string(), "invalid block: height mismatch");
    }

    #[test]
    fn consensus_error_wraps_validation_and_uses_same_message() {
        let v = ValidationError::Invalid("parent not found");
        let e: ConsensusError = v.into();
        assert_eq!(e.to_string(), "invalid block: parent not found");
    }

    #[test]
    fn consensus_error_display_storage() {
        let e = ConsensusError::Storage("missing parent block".to_string());
        assert_eq!(e.to_string(), "storage error: missing parent block");
    }

    #[test]
    fn consensus_error_display_other() {
        let e = ConsensusError::Other("timer failed".to_string());
        assert_eq!(e.to_string(), "consensus error: timer failed");
    }

    #[test]
    fn types_implement_std_error() {
        fn assert_is_error<E: StdError>() {}

        assert_is_error::<ValidationError>();
        assert_is_error::<ConsensusError>();
    }
}
