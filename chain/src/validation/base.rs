//! Base validity predicate (`V_base`-style) for blocks.
//!
//! This validator enforces cheap, deterministic invariants that do not
//! require access to external services, such as:
//!
//! - block size and transaction count limits,
//! - absence of duplicate `Aid` registrations within a single block.

use std::collections::HashSet;

use crate::consensus::config::ConsensusConfig;
use crate::consensus::error::ValidationError;
use crate::consensus::validator::BlockValidator;
use crate::types::{Aid, Block, Transaction};

/// Base validity predicate for blocks.
///
/// This struct is configured using [`ConsensusConfig`] and performs
/// purely block-local checks that are inexpensive to run.
#[derive(Clone, Debug)]
pub struct BaseValidity {
    max_block_txs: usize,
    max_block_size_bytes: usize,
}

impl BaseValidity {
    /// Constructs a new `BaseValidity` from the consensus configuration.
    pub fn new(cfg: &ConsensusConfig) -> Self {
        Self {
            max_block_txs: cfg.max_block_txs,
            max_block_size_bytes: cfg.max_block_size_bytes,
        }
    }

    fn check_tx_count(&self, block: &Block) -> Result<(), ValidationError> {
        let tx_count = block.txs.len();
        if tx_count > self.max_block_txs {
            return Err(ValidationError::Custom(format!(
                "block has {} txs, exceeds max_block_txs={}",
                tx_count, self.max_block_txs
            )));
        }
        Ok(())
    }

    fn check_block_size(&self, block: &Block) -> Result<(), ValidationError> {
        // Use the canonical bincode-2 encoding already defined on Block.
        let bytes = block.canonical_bytes();
        let size = bytes.len();
        if size > self.max_block_size_bytes {
            return Err(ValidationError::Custom(format!(
                "block size {} bytes exceeds max_block_size_bytes={}",
                size, self.max_block_size_bytes
            )));
        }
        Ok(())
    }

    fn check_duplicate_aids(&self, block: &Block) -> Result<(), ValidationError> {
        let mut seen: HashSet<Aid> = HashSet::new();

        for tx in &block.txs {
            if let Transaction::RegisterModel(tx_reg) = tx {
                if !seen.insert(tx_reg.aid) {
                    return Err(ValidationError::Custom(
                        "duplicate Aid in TxRegisterModel within the same block".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl BlockValidator for BaseValidity {
    fn validate(&self, block: &Block) -> Result<(), ValidationError> {
        self.check_tx_count(block)?;
        self.check_block_size(block)?;
        self.check_duplicate_aids(block)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AccountId, Aid, EvidenceHash, EvidenceRef, HASH_LEN, Hash256, Signature, Transaction,
        TxRegisterModel, WmProfile,
    };

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

    fn dummy_reg_tx(owner: AccountId, aid: Aid) -> Transaction {
        let tx = TxRegisterModel {
            owner,
            aid,
            evidence: dummy_evidence(3),
            fee: 0,
            nonce: 0,
            signature: Signature(vec![]),
        };
        Transaction::RegisterModel(tx)
    }

    fn dummy_block_with_txs(txs: Vec<Transaction>) -> crate::types::Block {
        use crate::types::{Block, BlockHash, Header};

        let header = Header {
            parent: BlockHash(Hash256([0u8; HASH_LEN])),
            height: 0,
            timestamp: 1_700_000_000,
            proposer: dummy_account(1),
            pos_proof: None,
        };

        Block { header, txs }
    }

    #[test]
    fn base_validity_accepts_small_block() {
        let cfg = ConsensusConfig {
            block_time_secs: 5,
            max_block_txs: 10,
            max_block_size_bytes: 1_000_000,
            allow_empty_blocks: true,
        };
        let v = BaseValidity::new(&cfg);

        let txs = vec![dummy_reg_tx(dummy_account(1), Aid(dummy_hash(2)))];
        let block = dummy_block_with_txs(txs);

        assert!(v.validate(&block).is_ok());
    }

    #[test]
    fn base_validity_rejects_too_many_txs() {
        let cfg = ConsensusConfig {
            block_time_secs: 5,
            max_block_txs: 1,
            max_block_size_bytes: 1_000_000,
            allow_empty_blocks: true,
        };
        let v = BaseValidity::new(&cfg);

        let txs = vec![
            dummy_reg_tx(dummy_account(1), Aid(dummy_hash(2))),
            dummy_reg_tx(dummy_account(2), Aid(dummy_hash(3))),
        ];
        let block = dummy_block_with_txs(txs);

        let err = v.validate(&block).unwrap_err();
        match err {
            ValidationError::Custom(msg) => {
                assert!(
                    msg.contains("exceeds max_block_txs"),
                    "unexpected message: {msg}"
                );
            }
            _ => panic!("unexpected error variant: {err:?}"),
        }
    }

    #[test]
    fn base_validity_rejects_duplicate_aids_in_block() {
        let cfg = ConsensusConfig {
            block_time_secs: 5,
            max_block_txs: 10,
            max_block_size_bytes: 1_000_000,
            allow_empty_blocks: true,
        };
        let v = BaseValidity::new(&cfg);

        let aid = Aid(dummy_hash(42));
        let txs = vec![
            dummy_reg_tx(dummy_account(1), aid),
            dummy_reg_tx(dummy_account(2), aid),
        ];
        let block = dummy_block_with_txs(txs);

        let err = v.validate(&block).unwrap_err();
        match err {
            ValidationError::Custom(msg) => {
                assert!(msg.contains("duplicate Aid"), "unexpected message: {msg}");
            }
            _ => panic!("unexpected error variant: {err:?}"),
        }
    }

    #[test]
    fn base_validity_rejects_oversized_block() {
        // Force a tiny max size so even a small block exceeds it.
        let cfg = ConsensusConfig {
            block_time_secs: 5,
            max_block_txs: 10,
            max_block_size_bytes: 1, // absurdly small
            allow_empty_blocks: true,
        };
        let v = BaseValidity::new(&cfg);

        let txs = vec![dummy_reg_tx(dummy_account(1), Aid(dummy_hash(2)))];
        let block = dummy_block_with_txs(txs);

        let err = v.validate(&block).unwrap_err();
        match err {
            ValidationError::Custom(msg) => {
                assert!(msg.contains("block size"), "unexpected message: {msg}");
            }
            _ => panic!("unexpected error variant: {err:?}"),
        }
    }
}
