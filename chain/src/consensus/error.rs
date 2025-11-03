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
