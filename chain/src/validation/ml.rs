//! ML-specific validity predicate (`V_auth` / `V_cons`-style) for blocks.
//!
//! This module defines a trait [`MlVerifier`] that abstracts over the
//! off-chain ML service, and a block validator [`MlValidity`] that:
//!
//! - extracts `ML(B)` = all `(Aid, EvidenceRef)` pairs in a block,
//! - deduplicates them within the block,
//! - calls the verifier for each pair, and
//! - fails the block if any verdict is negative.

use std::collections::HashSet;

use crate::consensus::error::ValidationError;
use crate::consensus::validator::BlockValidator;
use crate::types::{Aid, Block, EvidenceHash, EvidenceRef};

/// Result of an ML authenticity check for a single artefact.
#[derive(Clone, Debug)]
pub struct MlVerdict {
    /// Overall verdict: `true` if the artefact passes V_auth.
    pub ok: bool,
    /// Optional diagnostic statistics (trigger accuracy, etc.).
    pub trigger_acc: Option<f32>,
    pub feat_dist: Option<f32>,
    pub logit_stat: Option<f32>,
    pub latency_ms: Option<u64>,
}

/// Errors that can occur while contacting the ML verification service.
#[derive(Debug)]
pub enum MlError {
    /// Transport-level error (e.g. HTTP failure, timeout).
    Transport(String),
    /// The ML service returned a malformed or unexpected response.
    Protocol(String),
    /// The ML service actively refused to verify this artefact.
    Service(String),
}

/// Abstract ML verifier used by [`MlValidity`].
///
/// Implementations are responsible for contacting the external ML service
/// (e.g. via HTTP/gRPC) and performing the watermark-based authenticity
/// checks described in the thesis.
pub trait MlVerifier: Send + Sync {
    fn verify(&self, aid: &Aid, evidence: &EvidenceRef) -> Result<MlVerdict, MlError>;
}

/// Configuration options for [`MlValidity`].
#[derive(Clone, Debug)]
pub struct MlConfig {
    /// Maximum number of distinct artefacts per block we are willing to
    /// verify. Blocks exceeding this bound will be rejected to bound
    /// worst-case ML verification cost.
    pub max_artefacts_per_block: usize,
}

impl Default for MlConfig {
    fn default() -> Self {
        Self {
            max_artefacts_per_block: 1024,
        }
    }
}

/// ML-specific block validity predicate.
///
/// This validator is intentionally ignorant of consensus details; it only
/// cares about the artefacts referenced in a block and their authenticity
/// according to the provided [`MlVerifier`].
pub struct MlValidity<V> {
    cfg: MlConfig,
    verifier: V,
}

impl<V> MlValidity<V> {
    /// Constructs a new `MlValidity` from a verifier and configuration.
    pub fn new(verifier: V, cfg: MlConfig) -> Self {
        Self { cfg, verifier }
    }
}

impl<V> BlockValidator for MlValidity<V>
where
    V: MlVerifier,
{
    fn validate(&self, block: &Block) -> Result<(), ValidationError> {
        // Extract ML(B) = all (Aid, EvidenceRef) pairs from TxRegisterModel.
        let pairs = block.ml_pairs();

        // Deduplicate by (Aid, EvidenceHash) so we don't re-verify the same
        // logical artefact multiple times in a single block.
        let mut seen: HashSet<(Aid, EvidenceHash)> = HashSet::new();
        let mut unique_pairs = Vec::new();

        for (aid, evidence) in pairs {
            let key = (aid, evidence.evidence_hash);
            if seen.insert(key) {
                unique_pairs.push((aid, evidence));
            }
        }

        // Enforce per-block cap on ML artefacts.
        if unique_pairs.len() > self.cfg.max_artefacts_per_block {
            return Err(ValidationError::Custom(format!(
                "block references {} distinct ML artefacts, exceeds max_artefacts_per_block={}",
                unique_pairs.len(),
                self.cfg.max_artefacts_per_block
            )));
        }

        // Verify each unique artefact.
        for (aid, evidence) in unique_pairs {
            let verdict = self
                .verifier
                .verify(&aid, &evidence)
                .map_err(|e| ValidationError::Custom(format!("ML verifier error: {e:?}")))?;

            if !verdict.ok {
                return Err(ValidationError::Custom(
                    "ML authenticity check failed for artefact".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AccountId, Block, BlockHash, EvidenceHash, HASH_LEN, Hash256, Header, Transaction,
        TxRegisterModel, WmProfile,
    };

    struct DummyVerifier {
        // if true, all checks succeed; if false, all fail
        ok: bool,
    }

    impl MlVerifier for DummyVerifier {
        fn verify(&self, _aid: &Aid, _evidence: &EvidenceRef) -> Result<MlVerdict, MlError> {
            Ok(MlVerdict {
                ok: self.ok,
                trigger_acc: None,
                feat_dist: None,
                logit_stat: None,
                latency_ms: None,
            })
        }
    }

    fn dummy_hash(byte: u8) -> Hash256 {
        Hash256([byte; HASH_LEN])
    }

    fn dummy_account(byte: u8) -> AccountId {
        AccountId(dummy_hash(byte))
    }

    fn dummy_wm_profile() -> WmProfile {
        WmProfile {
            tau_input: 0.9,
            tau_feat: 0.1,
            logit_band_low: 0.02,
            logit_band_high: 0.05,
        }
    }

    fn dummy_evidence(byte: u8) -> EvidenceRef {
        EvidenceRef {
            scheme_id: format!("wm-test-{byte}"),
            evidence_hash: EvidenceHash(dummy_hash(byte)),
            wm_profile: dummy_wm_profile(),
        }
    }

    fn dummy_block_with_aids(aids: &[u8]) -> Block {
        let header = Header {
            parent: BlockHash(Hash256([0u8; HASH_LEN])),
            height: 0,
            timestamp: 1_700_000_000,
            proposer: dummy_account(1),
            pos_proof: None,
        };

        let txs = aids
            .iter()
            .map(|b| {
                let tx = TxRegisterModel {
                    owner: dummy_account(*b),
                    aid: Aid(dummy_hash(*b)),
                    evidence: dummy_evidence(*b),
                    fee: 0,
                    nonce: 0,
                    signature: crate::types::Signature(vec![]),
                };
                Transaction::RegisterModel(tx)
            })
            .collect();

        Block { header, txs }
    }

    #[test]
    fn ml_validity_accepts_when_verifier_ok() {
        let cfg = MlConfig::default();
        let verifier = DummyVerifier { ok: true };
        let v = MlValidity::new(verifier, cfg);

        let block = dummy_block_with_aids(&[1, 2, 3]);
        assert!(v.validate(&block).is_ok());
    }

    #[test]
    fn ml_validity_rejects_when_verifier_fails() {
        let cfg = MlConfig::default();
        let verifier = DummyVerifier { ok: false };
        let v = MlValidity::new(verifier, cfg);

        let block = dummy_block_with_aids(&[1, 2, 3]);
        let err = v.validate(&block).unwrap_err();
        match err {
            ValidationError::Custom(msg) => {
                assert!(
                    msg.contains("ML authenticity check failed"),
                    "unexpected message: {msg}"
                );
            }
            _ => panic!("unexpected error variant: {err:?}"),
        }
    }

    #[test]
    fn ml_validity_enforces_max_artefacts_per_block() {
        let cfg = MlConfig {
            max_artefacts_per_block: 1,
        };
        let verifier = DummyVerifier { ok: true };
        let v = MlValidity::new(verifier, cfg);

        let block = dummy_block_with_aids(&[1, 2]); // 2 distinct aids
        let err = v.validate(&block).unwrap_err();
        match err {
            ValidationError::Custom(msg) => {
                assert!(
                    msg.contains("exceeds max_artefacts_per_block"),
                    "unexpected message: {msg}"
                );
            }
            _ => panic!("unexpected error variant: {err:?}"),
        }
    }

    #[test]
    fn ml_validity_deduplicates_same_aid_and_evidence() {
        // max_artefacts_per_block == 1, but we include the same aid twice.
        let cfg = MlConfig {
            max_artefacts_per_block: 1,
        };
        let verifier = DummyVerifier { ok: true };
        let v = MlValidity::new(verifier, cfg);

        let header = Header {
            parent: BlockHash(Hash256([0u8; HASH_LEN])),
            height: 0,
            timestamp: 1_700_000_000,
            proposer: dummy_account(1),
            pos_proof: None,
        };

        let aid = Aid(dummy_hash(9));
        let evidence = dummy_evidence(9);

        let tx1 = TxRegisterModel {
            owner: dummy_account(1),
            aid,
            evidence: EvidenceRef {
                scheme_id: evidence.scheme_id.clone(),
                evidence_hash: evidence.evidence_hash,
                wm_profile: dummy_wm_profile(),
            },
            fee: 0,
            nonce: 0,
            signature: crate::types::Signature(vec![]),
        };

        let tx2 = TxRegisterModel {
            owner: dummy_account(2),
            aid,
            evidence,
            fee: 0,
            nonce: 1,
            signature: crate::types::Signature(vec![]),
        };

        let block = Block {
            header,
            txs: vec![
                Transaction::RegisterModel(tx1),
                Transaction::RegisterModel(tx2),
            ],
        };

        // Should be accepted because we deduplicate (aid, evidence_hash).
        assert!(v.validate(&block).is_ok());
    }
}
