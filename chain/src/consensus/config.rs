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
