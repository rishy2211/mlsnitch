//! Block validity predicates for the chain.
//!
//! This module implements concrete block validators that plug into the
//! consensus layer via [`crate::consensus::validator::BlockValidator`].
//!
//! It currently provides:
//!
//! - [`base::BaseValidity`]: cheap structural and size checks (V_base-ish).
//! - [`ml::MlValidity`]: ML-specific authenticity checks via a generic
//!   [`ml::MlVerifier`] interface.

pub mod base;
pub mod ml;

pub use base::BaseValidity;
