/// Consensus configuration parameters.
///
/// This includes both protocol-level knobs (e.g. target block time) and
/// implementation-level limits (e.g. maximum transactions per block).
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    /// Target block time in seconds for the simulator.
    pub block_time_secs: u64,
    /// Soft limit on the number of transactions per block.
    pub max_block_txs: usize,
    /// Soft limit on the total serialized size of a block, in bytes.
    pub max_block_size_bytes: usize,
    /// Whether to allow empty blocks when the transaction pool is empty.
    pub allow_empty_blocks: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            block_time_secs: 5,
            max_block_txs: 10_000,
            max_block_size_bytes: 1_000_000,
            allow_empty_blocks: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_expected() {
        let cfg = ConsensusConfig::default();

        assert_eq!(cfg.block_time_secs, 5);
        assert_eq!(cfg.max_block_txs, 10_000);
        assert_eq!(cfg.max_block_size_bytes, 1_000_000);
        assert!(cfg.allow_empty_blocks);
    }

    #[test]
    fn can_override_all_fields() {
        let cfg = ConsensusConfig {
            block_time_secs: 42,
            max_block_txs: 1_234,
            max_block_size_bytes: 512_000,
            allow_empty_blocks: false,
        };

        assert_eq!(cfg.block_time_secs, 42);
        assert_eq!(cfg.max_block_txs, 1_234);
        assert_eq!(cfg.max_block_size_bytes, 512_000);
        assert!(!cfg.allow_empty_blocks);
    }

    #[test]
    fn consensus_config_is_clone_and_debug() {
        fn assert_clone_debug<T: Clone + core::fmt::Debug>() {}

        assert_clone_debug::<ConsensusConfig>();
    }
}
