//! Block proposal logic.
//!
//! The proposer is responsible for assembling a candidate block on top of
//! the current tip, given a view of the chain and a transaction pool.

use crate::types::{AccountId, Block, BlockHash, HASH_LEN, Hash256, Header, Transaction};

use super::config::ConsensusConfig;
use super::store::BlockStore;

/// Abstract transaction pool interface.
///
/// Consensus does not care how transactions are stored or gossiped; it only
/// needs a way to ask for a batch of transactions that fit into a block.
pub trait TxPool {
    /// Selects a batch of transactions for inclusion in a block.
    ///
    /// Implementations should respect the `max_txs` and `max_bytes` hints
    /// as soft limits (they may choose fewer transactions but should not
    /// exceed the size bound).
    fn select_for_block(&mut self, max_txs: usize, max_bytes: usize) -> Vec<Transaction>;
}

/// Configurable block proposer.
///
/// This struct is deliberately stateless with respect to the chain; it
/// uses a [`BlockStore`] and [`TxPool`] provided at call time.
#[derive(Clone, Debug)]
pub struct Proposer {
    pub max_block_txs: usize,
    pub max_block_size_bytes: usize,
    pub allow_empty_blocks: bool,
}

impl Proposer {
    /// Constructs a proposer from a [`ConsensusConfig`].
    pub fn from_config(cfg: &ConsensusConfig) -> Self {
        Self {
            max_block_txs: cfg.max_block_txs,
            max_block_size_bytes: cfg.max_block_size_bytes,
            allow_empty_blocks: cfg.allow_empty_blocks,
        }
    }

    /// Builds a new block on top of the current tip.
    ///
    /// This does not perform validation or persistence; callers should pass
    /// the resulting block into the consensus engine for validation and
    /// import.
    pub fn build_block<S, P>(
        &self,
        store: &S,
        proposer: AccountId,
        tx_pool: &mut P,
        timestamp: u64,
    ) -> Block
    where
        S: BlockStore,
        P: TxPool,
    {
        let (parent_hash, next_height) = match store.tip() {
            Some(tip_hash) => match store.get_block(&tip_hash) {
                Some(tip_block) => (tip_hash, tip_block.header.height + 1),
                None => {
                    // Tip is set but block is missing: treat as no tip.
                    (BlockHash(Hash256([0u8; HASH_LEN])), 0)
                }
            },
            None => {
                // No tip yet: this is the first block (height 0).
                (BlockHash(Hash256([0u8; HASH_LEN])), 0)
            }
        };

        let mut txs = tx_pool.select_for_block(self.max_block_txs, self.max_block_size_bytes);

        if txs.is_empty() && !self.allow_empty_blocks {
            // If empty blocks are disallowed, we still produce a header-only
            // block with no transactions; higher layers can decide to skip
            // proposing such blocks if desired.
            txs = Vec::new();
        }

        let header = Header {
            parent: parent_hash,
            height: next_height,
            timestamp,
            proposer,
            pos_proof: None,
        };

        Block { header, txs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposer_from_config_copies_limits() {
        let cfg = ConsensusConfig {
            block_time_secs: 7,
            max_block_txs: 1234,
            max_block_size_bytes: 512_000,
            allow_empty_blocks: false,
        };

        let p = Proposer::from_config(&cfg);

        assert_eq!(p.max_block_txs, cfg.max_block_txs);
        assert_eq!(p.max_block_size_bytes, cfg.max_block_size_bytes);
        assert_eq!(p.allow_empty_blocks, cfg.allow_empty_blocks);
    }

    #[test]
    fn proposer_trait_bounds() {
        fn assert_bounds<T: Clone + core::fmt::Debug>() {}
        assert_bounds::<Proposer>();
    }

    #[test]
    fn build_block_signature_is_stable() {
        // This never runs; it's just a compile-time check that the
        // generics and return type of `build_block` stay as expected.
        fn _assert<S, P>(store: &S, tx_pool: &mut P, proposer: &Proposer, id: AccountId, ts: u64)
        where
            S: BlockStore,
            P: TxPool,
        {
            let _block: Block = proposer.build_block(store, id, tx_pool, ts);
        }
    }
}
