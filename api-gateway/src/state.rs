//! Shared application state and transaction pool implementation.

use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::Mutex;

use chain::{AccountId, DefaultConsensusEngine, MetricsRegistry, Transaction, TxPool};

/// Simple in-memory transaction pool backed by a FIFO queue.
///
/// HTTP handlers push transactions into the queue; the block producer
/// drains them when constructing blocks.
#[derive(Default)]
pub struct QueuedTxPool {
    queue: VecDeque<Transaction>,
}

impl QueuedTxPool {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Enqueues a new transaction to be included in a future block.
    pub fn push(&mut self, tx: Transaction) {
        self.queue.push_back(tx);
    }
}

impl TxPool for QueuedTxPool {
    fn select_for_block(&mut self, max_txs: usize, _max_bytes: usize) -> Vec<Transaction> {
        let take = max_txs.min(self.queue.len());
        self.queue.drain(0..take).collect()
    }
}

/// Shared state held by the API and background tasks.
///
/// This is wrapped in an [`Arc`] and passed to request handlers via Axum's
/// `State` extractor.
pub struct AppState {
    /// Embedded consensus engine (storage + validators + fork choice).
    pub engine: Mutex<DefaultConsensusEngine>,
    /// Transaction pool feeding the proposer.
    pub tx_pool: Mutex<QueuedTxPool>,
    /// Proposer identity used by the block producer loop.
    pub proposer_id: AccountId,
    /// Metrics registry shared between consensus and the API.
    pub metrics: Arc<MetricsRegistry>,
}

/// Thread-safe alias for `AppState`.
pub type SharedState = Arc<AppState>;
