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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EvidenceHash, HASH_LEN, Hash256, WmProfile};

    #[test]
    fn construct_metadata_compiles_and_fields_are_accessible() {
        let aid = Aid(Hash256([1u8; HASH_LEN]));
        let owner = AccountId(Hash256([2u8; HASH_LEN]));

        let wm_profile = WmProfile {
            tau_input: 0.9,
            tau_feat: 0.1,
            logit_band_low: 0.02,
            logit_band_high: 0.05,
        };

        let evidence = EvidenceRef {
            scheme_id: "multi_factor_v1".to_string(),
            evidence_hash: EvidenceHash(Hash256([3u8; HASH_LEN])),
            wm_profile,
        };

        let meta = ArtefactMetadata {
            aid,
            owner,
            evidence,
            registered_at: 42,
        };

        assert_eq!(meta.registered_at, 42);
        assert_eq!(meta.aid.0.as_bytes(), &[1u8; HASH_LEN]);
        assert_eq!(meta.owner.0.as_bytes(), &[2u8; HASH_LEN]);
    }

    #[test]
    fn serde_roundtrip_preserves_fields() {
        let aid = Aid(Hash256([9u8; HASH_LEN]));
        let owner = AccountId(Hash256([8u8; HASH_LEN]));

        let wm_profile = WmProfile {
            tau_input: 0.95,
            tau_feat: 0.05,
            logit_band_low: 0.01,
            logit_band_high: 0.04,
        };

        let evidence = EvidenceRef {
            scheme_id: "multi_factor_v1".to_string(),
            evidence_hash: EvidenceHash(Hash256([7u8; HASH_LEN])),
            wm_profile,
        };

        let original = ArtefactMetadata {
            aid,
            owner,
            evidence,
            registered_at: 123,
        };

        let json = serde_json::to_string(&original).expect("serialize metadata to json");
        let decoded: ArtefactMetadata =
            serde_json::from_str(&json).expect("deserialize metadata from json");

        assert_eq!(decoded.registered_at, original.registered_at);
        assert_eq!(decoded.aid.0.as_bytes(), original.aid.0.as_bytes());
        assert_eq!(decoded.owner.0.as_bytes(), original.owner.0.as_bytes());
        assert_eq!(
            decoded.evidence.evidence_hash.0.as_bytes(),
            original.evidence.evidence_hash.0.as_bytes()
        );
        assert_eq!(decoded.evidence.scheme_id, original.evidence.scheme_id);
    }
}
