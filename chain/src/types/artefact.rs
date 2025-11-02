//! Types for registered ML artefacts.
//!
//! This module defines the metadata that is stored in chain state for each
//! registered ML model artefact. The metadata connects:
//!
//! - a content-addressed artefact identifier (`Aid`),
//! - an owning account (`AccountId`),
//! - and watermark / authenticity evidence (`EvidenceRef`),
//!
//! together with the block height at which the artefact was first accepted.
//!
use serde::{Deserialize, Serialize};

use super::{AccountId, Aid, EvidenceRef};

/// Metadata stored in state for a registered ML artefact.
///
/// An `ArtefactMetadata` entry is created when a `TxRegisterModel`
/// transaction is successfully included in a block. It allows later
/// transactions to refer to an artefact via its [`Aid`] without having
/// to repeat ownership or watermark evidence on-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtefactMetadata {
    /// Content-addressed artefact identifier (hash of model bytes).
    ///
    /// This is usually `Aid::from_model_bytes(model_bytes)` where
    /// `model_bytes` is a canonical encoding of the artefact.
    pub aid: Aid,

    /// Current owner of the artefact.
    ///
    /// Ownership is expressed as an [`AccountId`] derived from the
    /// owner's Dilithium / ML-DSA public key. Transfer transactions
    /// update this field while keeping the `aid` stable.
    pub owner: AccountId,

    /// Authenticity evidence reference used when this artefact was accepted.
    ///
    /// This ties the artefact to the watermarking scheme and parameters
    /// that were in force at registration time. The actual evidence lives
    /// off-chain; the chain stores only a stable [`EvidenceRef`].
    pub evidence: EvidenceRef,

    /// Height at which the artefact was first accepted into the chain.
    ///
    /// This is the block height of the first successful registration and
    /// never decreases. It can be used for auditing, replay protection,
    /// or enforcing policies such as “only models registered before
    /// height _H_ are allowed in a given context”.
    pub registered_at: u64,
}
