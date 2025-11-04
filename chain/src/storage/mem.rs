//! In-memory block store.
//!
//! This implementation is useful for unit tests, benchmarks, and small
//! devnets. It keeps all blocks in a `HashMap` keyed by `BlockHash` and
//! tracks the current tip separately.

use std::collections::HashMap;

use crate::consensus::store::BlockStore;
use crate::types::{Block, BlockHash};

/// In-memory implementation of [`BlockStore`].
#[derive(Default)]
pub struct InMemoryBlockStore {
    blocks: HashMap<BlockHash, Block>,
    tip: Option<BlockHash>,
}

impl InMemoryBlockStore {
    /// Creates a new, empty in-memory block store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of blocks currently stored.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Returns `true` if no blocks are stored.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BlockHash, HASH_LEN, Hash256, Header};

    fn dummy_hash(byte: u8) -> Hash256 {
        Hash256([byte; HASH_LEN])
    }

    fn dummy_block(height: u64) -> Block {
        use crate::types::{AccountId, Block};

        let header = Header {
            parent: BlockHash(dummy_hash(0)),
            height,
            timestamp: 1_700_000_000 + height,
            proposer: AccountId(dummy_hash(1)),
            pos_proof: None,
        };

        Block {
            header,
            txs: Vec::new(),
        }
    }

    #[test]
    fn put_and_get_block_roundtrip() {
        let mut store = InMemoryBlockStore::new();
        let block = dummy_block(0);
        let hash = block.compute_hash();

        store.put_block(block.clone());
        let fetched = store.get_block(&hash).expect("block should be present");

        assert_eq!(fetched.header.height, 0);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn tip_is_tracked_separately_from_blocks() {
        let mut store = InMemoryBlockStore::new();
        let block = dummy_block(5);
        let hash = block.compute_hash();

        store.put_block(block);
        assert!(store.tip().is_none());

        store.set_tip(hash);
        let tip = store.tip().expect("tip should be set");
        assert_eq!(tip.0.as_bytes(), hash.0.as_bytes());
    }
}
