//! Fork-choice rule for selecting the best chain.

use crate::types::{Block, BlockHash};

use super::store::BlockStore;

/// Abstraction over fork-choice rules.
///
/// Given the current tip (if any) and a candidate block, a fork-choice
/// implementation decides whether the candidate should become the new tip.
pub trait ForkChoice {
    /// Returns `true` if the candidate block should replace the current tip.
    fn should_update_tip<S: BlockStore>(
        &self,
        store: &S,
        current_tip: Option<BlockHash>,
        candidate: &Block,
    ) -> bool;
}

/// Simple "longest chain by height" fork choice.
///
/// - If there is no current tip, the candidate always becomes the tip.
/// - If the candidate's height is strictly greater than the tip's height,
///   the candidate becomes the tip.
/// - If the heights are equal or lower, the tip remains unchanged.
#[derive(Clone, Copy, Debug, Default)]
pub struct LongestChainForkChoice;

impl ForkChoice for LongestChainForkChoice {
    fn should_update_tip<S: BlockStore>(
        &self,
        store: &S,
        current_tip: Option<BlockHash>,
        candidate: &Block,
    ) -> bool {
        let new_height = candidate.header.height;

        match current_tip {
            None => true,
            Some(tip_hash) => match store.get_block(&tip_hash) {
                Some(tip_block) => new_height > tip_block.header.height,
                None => {
                    // Tip block missing: treat storage as corrupted and allow
                    // the candidate to become the new tip.
                    true
                }
            },
        }
    }
}
