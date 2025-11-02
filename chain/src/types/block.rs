// chain/src/types/block.rs

//! Block types and hashing.
//!
//! This module defines the core block data structures used by the chain,
//! together with a canonical hashing routine and helpers for extracting
//! ML-related information from a block.
//!
//! Serialization is done with **bincode 2** using the `serde` integration
//! (`bincode::serde::encode_to_vec`) and an explicit `standard()` config.
//! The same canonical encoding is used everywhere we need block bytes.

use serde::{Deserialize, Serialize};

use super::{AccountId, Aid, EvidenceRef, Hash256, Transaction};

/// Strongly-typed block hash.
///
/// This is the content hash of a [`Block`], computed as a BLAKE3-256
/// digest over the canonical bincode-2 serialization of the block.
/// Wrapping the underlying [`Hash256`] avoids passing raw byte arrays
/// around in public APIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BlockHash(pub Hash256);

/// Block header: minimal consensus fields.
///
/// The header carries enough information to link blocks, order them,
/// and attribute them to a proposer. Additional consensus-specific
/// fields can be added over time (e.g. VRF outputs, PoS proofs).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Header {
    /// Hash of the parent block in the canonical chain.
    pub parent: BlockHash,

    /// Height of this block, starting from 0 or 1 at genesis.
    pub height: u64,

    /// Wall-clock timestamp of the block, in seconds since Unix epoch.
    ///
    /// This is primarily used for observability and coarse ordering.
    /// Consensus rules may constrain this (e.g. bounded drift).
    pub timestamp: u64,

    /// Account that proposed this block.
    ///
    /// The proposer is derived from a Dilithium / ML-DSA public key and
    /// may be used for rewards, slashing, or accountability.
    pub proposer: AccountId,

    /// Placeholder for PoS proof / VRF output etc.
    ///
    /// In a full PoS implementation, this will carry whatever randomness
    /// or eligibility proof is required by the consensus protocol.
    pub pos_proof: Option<Vec<u8>>,
}

/// Block = header + list of transactions.
///
/// A block is the atomic unit of consensus in the chain. It bundles a
/// [`Header`] with an ordered list of [`Transaction`]s.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    /// Header containing linking, ordering, and proposer information.
    pub header: Header,
    /// Ordered list of transactions included in this block.
    pub txs: Vec<Transaction>,
}

impl Block {
    /// Returns the canonical byte representation of this block.
    ///
    /// This uses **bincode 2** with the `standard()` configuration and
    /// the `serde` integration. All hashing, signing, and network
    /// encoding that depend on a "canonical" form should go through
    /// this method to avoid format drift.
    ///
    /// # Panics
    ///
    /// Panics if encoding fails. This is considered a programming
    /// error, because all fields are required to be serializable.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Explicit config to avoid relying on any implicit defaults.
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg)
            .expect("Block should always be serializable with bincode 2 + serde")
    }

    /// Computes a canonical BLAKE3-256 hash for this block.
    ///
    /// The block is serialized with [`bincode`] v2 using
    /// [`Block::canonical_bytes`] and the resulting bytes are hashed
    /// with [`Hash256::compute`]. This must remain stable across nodes
    /// for consensus to work correctly.
    pub fn compute_hash(&self) -> BlockHash {
        let bytes = self.canonical_bytes();
        BlockHash(Hash256::compute(&bytes))
    }

    /// Extracts all `(aid, evidence)` pairs from `TxRegisterModel` in this block.
    ///
    /// This is the set `ML(B)` used by the `MlValidity` predicate to drive
    /// the `V_auth` check in the consensus protocol, i.e. all model artefacts
    /// that are newly registered in this block.
    pub fn ml_pairs(&self) -> Vec<(Aid, EvidenceRef)> {
        self.txs
            .iter()
            .filter_map(|tx| match tx {
                Transaction::RegisterModel(tx_reg) => Some((tx_reg.aid, tx_reg.evidence.clone())),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EvidenceHash, WmProfile};

    #[test]
    fn block_hash_is_deterministic() {
        // Tiny smoke test to ensure canonical_bytes + compute_hash are stable
        // for the same logical block.
        let dummy_hash = Hash256([1u8; super::super::HASH_LEN]);
        let parent = BlockHash(dummy_hash);

        let header = Header {
            parent,
            height: 1,
            timestamp: 1_700_000_000,
            proposer: AccountId(Hash256([2u8; super::super::HASH_LEN])),
            pos_proof: None,
        };

        let wm_profile = WmProfile {
            tau_input: 0.9,
            tau_feat: 0.1,
            logit_band_low: 0.02,
            logit_band_high: 0.05,
        };

        let evidence = EvidenceRef {
            scheme_id: "multi_factor_v1".to_string(),
            evidence_hash: EvidenceHash(Hash256([3u8; super::super::HASH_LEN])),
            wm_profile,
        };

        let aid = Aid(Hash256([4u8; super::super::HASH_LEN]));

        let tx = Transaction::RegisterModel(crate::types::tx::TxRegisterModel {
            owner: AccountId(Hash256([5u8; super::super::HASH_LEN])),
            aid,
            evidence,
            fee: 0,
            nonce: 0,
            signature: crate::types::Signature(vec![]),
        });

        let block = Block {
            header,
            txs: vec![tx],
        };

        let h1 = block.compute_hash();
        let h2 = block.compute_hash();

        assert_eq!(h1.0.as_bytes(), h2.0.as_bytes());
    }
}
