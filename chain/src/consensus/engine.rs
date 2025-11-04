//! High-level consensus engine orchestration.
//!
//! The consensus engine wires together:
//!
//! - a [`BlockStore`] for persistence,
//! - a [`BlockValidator`] for `V_base` and `V_cons`,
//! - a [`ForkChoice`] implementation, and
//! - a [`Proposer`] for block construction.
//!
//! It exposes methods to propose new blocks (for local leadership) and to
//! import blocks (from local or remote proposers) into the canonical chain.

use crate::types::{AccountId, Block, BlockHash};

use super::config::ConsensusConfig;
use super::error::ConsensusError;
use super::fork_choice::ForkChoice;
use super::proposer::{Proposer, TxPool};
use super::store::BlockStore;
use super::validator::BlockValidator;

/// Fully-configurable consensus engine.
///
/// This struct is generic over:
///
/// - `S`: storage backend implementing [`BlockStore`],
/// - `V`: block validator implementing [`BlockValidator`],
/// - `F`: fork-choice rule implementing [`ForkChoice`].
pub struct ConsensusEngine<S, V, F> {
    pub config: ConsensusConfig,
    store: S,
    validator: V,
    fork_choice: F,
    proposer: Proposer,
}

impl<S, V, F> ConsensusEngine<S, V, F>
where
    S: BlockStore,
    V: BlockValidator,
    F: ForkChoice,
{
    /// Creates a new consensus engine.
    pub fn new(config: ConsensusConfig, store: S, validator: V, fork_choice: F) -> Self {
        let proposer = Proposer::from_config(&config);
        Self {
            config,
            store,
            validator,
            fork_choice,
            proposer,
        }
    }

    /// Returns a reference to the underlying block store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Returns a mutable reference to the underlying block store.
    ///
    /// This is mainly useful for tests and tooling; consensus logic should
    /// normally go through [`import_block`].
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Returns the hash of the current tip of the best chain, if any.
    pub fn tip(&self) -> Option<BlockHash> {
        self.store.tip()
    }

    /// Returns the tip block, if any.
    pub fn tip_block(&self) -> Option<Block> {
        self.tip().and_then(|h| self.store.get_block(&h))
    }

    /// Proposes a new block using the embedded [`Proposer`].
    ///
    /// This:
    /// 1. Builds a candidate block on top of the current tip.
    /// 2. Validates and imports it (so it updates the fork choice if valid).
    /// 3. Returns the new block hash and the block itself.
    pub fn propose_block<P>(
        &mut self,
        proposer_id: AccountId,
        tx_pool: &mut P,
        timestamp: u64,
    ) -> Result<(BlockHash, Block), ConsensusError>
    where
        P: TxPool,
    {
        let block = self
            .proposer
            .build_block(&self.store, proposer_id, tx_pool, timestamp);
        let hash = self.import_block(block.clone())?;
        Ok((hash, block))
    }

    /// Validates and imports a block into the chain.
    ///
    /// This method is used both for locally proposed blocks and blocks
    /// received from the network. It performs:
    ///
    /// - block validation via the configured [`BlockValidator`],
    /// - persistence via [`BlockStore`],
    /// - fork-choice update via the configured [`ForkChoice`].
    pub fn import_block(&mut self, block: Block) -> Result<BlockHash, ConsensusError> {
        // 1. Run validity predicates (V_base + V_cons).
        self.validator
            .validate(&block)
            .map_err(ConsensusError::from)?;

        // 2. Compute the block's hash and height.
        let new_hash = block.compute_hash();

        // 3. Decide whether this block should become the new tip.
        let current_tip = self.store.tip();
        let should_update_tip =
            self.fork_choice
                .should_update_tip(&self.store, current_tip, &block);

        // 4. Persist the block.
        self.store.put_block(block);

        // 5. Update tip if fork-choice prefers the new block.
        if should_update_tip {
            self.store.set_tip(new_hash);
        }

        Ok(new_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Aid, Block, BlockHash, EvidenceHash, EvidenceRef, HASH_LEN, Hash256, Header, Transaction,
        WmProfile,
    };
    use std::collections::HashMap;

    use super::super::fork_choice::LongestChainForkChoice;
    use super::super::store::BlockStore;
    use super::super::validator::AcceptAllValidator;

    /// Simple in-memory block store for tests and small simulations.
    struct InMemoryBlockStore {
        blocks: HashMap<BlockHash, Block>,
        tip: Option<BlockHash>,
    }

    impl InMemoryBlockStore {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
                tip: None,
            }
        }
    }

    impl BlockStore for InMemoryBlockStore {
        fn get_block(&self, hash: &BlockHash) -> Option<Block> {
            self.blocks.get(hash).cloned()
        }

        fn put_block(&mut self, block: Block) {
            let hash = block.compute_hash();
            self.blocks.insert(hash, block);
        }

        fn tip(&self) -> Option<BlockHash> {
            self.tip
        }

        fn set_tip(&mut self, hash: BlockHash) {
            self.tip = Some(hash);
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

    /// Minimal TxPool implementation for tests.
    struct TestTxPool {
        txs: Vec<Transaction>,
    }

    impl TestTxPool {
        fn new(txs: Vec<Transaction>) -> Self {
            Self { txs }
        }
    }

    impl super::super::proposer::TxPool for TestTxPool {
        fn select_for_block(&mut self, max_txs: usize, _max_bytes: usize) -> Vec<Transaction> {
            let take = max_txs.min(self.txs.len());
            self.txs.drain(0..take).collect()
        }
    }

    /// Build a minimal RegisterModel tx just to get something in the block.
    fn dummy_register_tx(owner_byte: u8, aid_byte: u8) -> Transaction {
        let owner = dummy_account(owner_byte);
        let aid = Aid(dummy_hash(aid_byte));

        let wm_profile = dummy_wm_profile();
        let evidence_ref = EvidenceRef {
            scheme_id: "wm-test".to_string(),
            evidence_hash: EvidenceHash(dummy_hash(3)),
            wm_profile,
        };

        let tx_reg = crate::types::tx::TxRegisterModel {
            owner,
            aid,
            evidence: evidence_ref,
            fee: 0,
            nonce: 0,
            signature: crate::types::Signature(vec![]),
        };

        Transaction::RegisterModel(tx_reg)
    }

    #[test]
    fn propose_and_import_block_updates_tip() {
        let cfg = ConsensusConfig {
            block_time_secs: 5,
            max_block_txs: 100,
            max_block_size_bytes: 1_000_000,
            allow_empty_blocks: true,
        };
        let store = InMemoryBlockStore::new();
        let validator = AcceptAllValidator;
        let fork_choice = LongestChainForkChoice::default();

        let mut engine = ConsensusEngine::new(cfg, store, validator, fork_choice);

        let proposer_id = dummy_account(1);
        let txs = vec![dummy_register_tx(1, 2)];
        let mut tx_pool = TestTxPool::new(txs);

        let (hash, block) = engine
            .propose_block(proposer_id, &mut tx_pool, 1_700_000_000)
            .expect("proposal should succeed");

        assert_eq!(block.header.height, 0);

        let tip = engine.tip().expect("tip should be set");
        assert_eq!(tip.0.as_bytes(), hash.0.as_bytes());
    }

    #[test]
    fn longest_chain_fork_choice_prefers_higher_height() {
        let cfg = ConsensusConfig {
            block_time_secs: 5,
            max_block_txs: 100,
            max_block_size_bytes: 1_000_000,
            allow_empty_blocks: true,
        };
        let store = InMemoryBlockStore::new();
        let validator = AcceptAllValidator;
        let fork_choice = LongestChainForkChoice::default();

        let mut engine = ConsensusEngine::new(cfg, store, validator, fork_choice);

        let proposer_id = dummy_account(1);

        // First block via propose_block.
        let mut tx_pool = TestTxPool::new(vec![dummy_register_tx(1, 2)]);
        let (h0, _) = engine
            .propose_block(proposer_id, &mut tx_pool, 1_700_000_000)
            .expect("b0 valid");

        // Competing block at height 0 built manually (not via proposer).
        let alt_block = {
            let header = Header {
                parent: BlockHash(Hash256([0u8; HASH_LEN])),
                height: 0,
                timestamp: 1_700_000_001,
                proposer: proposer_id,
                pos_proof: None,
            };
            Block {
                header,
                txs: vec![dummy_register_tx(3, 4)],
            }
        };
        let alt_hash = alt_block.compute_hash();
        engine
            .import_block(alt_block)
            .expect("alternate block should also be valid");

        // Tip should still point to the original height-0 block (ties stay).
        let tip1 = engine.tip().unwrap();
        assert_eq!(tip1.0.as_bytes(), h0.0.as_bytes());

        // Now propose a new block on top of the current tip (height 1).
        let mut tx_pool2 = TestTxPool::new(vec![dummy_register_tx(5, 6)]);
        let (h1, _) = engine
            .propose_block(proposer_id, &mut tx_pool2, 1_700_000_010)
            .expect("b1 valid");

        let tip2 = engine.tip().unwrap();
        assert_eq!(tip2.0.as_bytes(), h1.0.as_bytes());
        assert_ne!(tip2.0.as_bytes(), alt_hash.0.as_bytes());
    }
}
