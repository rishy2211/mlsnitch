//! Storage abstraction used by the consensus engine.

use crate::types::{Block, BlockHash};

/// Abstract storage interface used by the consensus engine.
///
/// Implementations can be backed by in-memory maps, RocksDB, etc. The
/// interface is intentionally small: consensus only needs get/put and
/// a notion of the current tip.
pub trait BlockStore {
    /// Fetches a block by hash, if present.
    fn get_block(&self, hash: &BlockHash) -> Option<Block>;

    /// Persists a block.
    fn put_block(&mut self, block: Block);

    /// Returns the hash of the current tip of the best chain, if any.
    fn tip(&self) -> Option<BlockHash>;

    /// Updates the current tip of the best chain.
    fn set_tip(&mut self, hash: BlockHash);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HASH_LEN, Hash256};

    /// Minimal dummy store; good for checking trait-object use and basic
    /// tip semantics without caring about real blocks.
    #[derive(Default)]
    struct DummyStore {
        tip: Option<BlockHash>,
    }

    impl BlockStore for DummyStore {
        fn get_block(&self, _hash: &BlockHash) -> Option<Block> {
            None
        }

        fn put_block(&mut self, _block: Block) {
            // no-op
        }

        fn tip(&self) -> Option<BlockHash> {
            self.tip
        }

        fn set_tip(&mut self, hash: BlockHash) {
            self.tip = Some(hash);
        }
    }

    #[test]
    fn block_store_trait_is_object_safe() {
        fn use_trait_object(store: &mut dyn BlockStore) {
            // Just make sure we can call trait methods via a trait object.
            let _ = store.tip();
        }

        let mut store = DummyStore::default();
        use_trait_object(&mut store);
    }

    #[test]
    fn dummy_store_tracks_tip_hash() {
        let mut store = DummyStore::default();
        assert!(store.tip().is_none());

        let zero_hash = BlockHash(Hash256([0u8; HASH_LEN]));
        store.set_tip(zero_hash);
        let tip = store.tip();

        assert!(tip.is_some());
        assert_eq!(tip.unwrap().0.0, [0u8; HASH_LEN]);
    }
}
