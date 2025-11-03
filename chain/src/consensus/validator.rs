//! Block validity predicates used by consensus.

use crate::types::Block;

use super::error::ValidationError;

/// Pluggable validity predicate for blocks.
///
/// Implementations should be deterministic and side-effect free. They can
/// encapsulate base validity (`V_base`) as well as extended ML validity
/// (`V_cons`) by composing multiple checks into a single call.
pub trait BlockValidator {
    fn validate(&self, block: &Block) -> Result<(), ValidationError>;
}

/// A trivial validator that accepts every block.
///
/// Useful for tests and for isolating consensus logic while the real
/// validity predicates are being developed.
pub struct AcceptAllValidator;

impl BlockValidator for AcceptAllValidator {
    fn validate(&self, _block: &Block) -> Result<(), ValidationError> {
        Ok(())
    }
}

/// A validator that composes two other validators.
///
/// This is a convenience to keep base and ML-specific checks modular:
/// `CombinedValidator { base, ml }` will run `base.validate` and then
/// `ml.validate`, failing fast on the first error.
pub struct CombinedValidator<B, M> {
    pub base: B,
    pub ml: M,
}

impl<B, M> CombinedValidator<B, M> {
    pub fn new(base: B, ml: M) -> Self {
        Self { base, ml }
    }
}

impl<B, M> BlockValidator for CombinedValidator<B, M>
where
    B: BlockValidator,
    M: BlockValidator,
{
    fn validate(&self, block: &Block) -> Result<(), ValidationError> {
        self.base.validate(block)?;
        self.ml.validate(block)?;
        Ok(())
    }
}
