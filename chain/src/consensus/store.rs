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
