// src/main.rs
//
// Minimal demo node that wires up the chain library:
//
// - RocksDB-backed storage
// - Base + ML validity (with HTTP ML verifier)
// - Longest-chain fork choice
// - Prometheus metrics exporter on /metrics
// - Simple loop that proposes (currently empty) blocks at a fixed interval.

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hex;
use tokio;

use chain::{
    // Domain types
    AccountId,
    // Validation stack
    BaseValidity,
    // Top-level config
    ChainConfig,
    CombinedValidator,
    // Consensus engine + fork choice
    ConsensusEngine,
    DefaultForkChoice,
    Hash256,
    HttpMlVerifier,
    // Metrics
    MetricsRegistry,
    MlConfig,
    MlValidity,
    // Storage backend
    RocksDbBlockStore,
    Transaction,
    TxPool,
    run_prometheus_http_server,
};

#[tokio::main]
async fn main() {
    if let Err(err) = run_node().await {
        eprintln!("fatal error: {err}");
        std::process::exit(1);
    }
}

async fn run_node() -> Result<(), String> {
    // For now, just use defaults. Later you can load from a file/CLI/env.
    let cfg = ChainConfig::default();

    // ---------------------------
    // Metrics registry + exporter
    // ---------------------------

    let metrics = Arc::new(
        MetricsRegistry::new()
            .map_err(|e| format!("failed to initialise metrics registry: {e}"))?,
    );

    if cfg.metrics.enabled {
        let metrics_clone = metrics.clone();
        let addr = cfg.metrics.listen_addr;
        tokio::spawn(async move {
            if let Err(e) = run_prometheus_http_server(metrics_clone, addr).await {
                eprintln!("metrics HTTP server error: {e}");
            }
        });
        eprintln!("metrics exporter listening on http://{}/metrics", addr);
    }

    // ---------------------------
    // Storage backend (RocksDB)
    // ---------------------------

    let store = RocksDbBlockStore::open(&cfg.storage).map_err(|e| {
        format!(
            "failed to open RocksDB store at {}: {e:?}",
            cfg.storage.path
        )
    })?;

    // ---------------------------
    // ML verifier client (HTTP)
    // ---------------------------

    let ml_verifier = HttpMlVerifier::new(cfg.ml_client.base_url.clone(), cfg.ml_client.timeout)
        .map_err(|e| format!("failed to create HttpMlVerifier: {e:?}"))?;

    // ---------------------------
    // Block validators (base + ML)
    // ---------------------------

    let base_validity = BaseValidity::new(&cfg.consensus);
    let ml_validity = MlValidity::new(ml_verifier, MlConfig::default());
    let validator = CombinedValidator::new(base_validity, ml_validity);

    // ---------------------------
    // Fork choice + engine
    // ---------------------------

    let fork_choice = DefaultForkChoice::default();

    let mut engine: ConsensusEngine<_, _, _> =
        ConsensusEngine::new(cfg.consensus.clone(), store, validator, fork_choice);

    // ---------------------------
    // Proposer identity (demo)
    // ---------------------------

    // In a real node, this would come from a Dilithium keypair. For now we
    // derive a deterministic AccountId from a fixed byte string.
    let proposer_id = {
        let seed = b"demo-proposer-public-key";
        AccountId(Hash256::compute(seed))
    };

    // ---------------------------
    // Simple transaction pool (empty)
    // ---------------------------

    struct EmptyTxPool;

    impl TxPool for EmptyTxPool {
        fn select_for_block(&mut self, _max_txs: usize, _max_bytes: usize) -> Vec<Transaction> {
            Vec::new()
        }
    }

    let mut tx_pool = EmptyTxPool;
    let block_interval = cfg.consensus.block_time_secs;

    eprintln!(
        "starting node with block_time_secs={} (empty TxPool)",
        block_interval
    );

    // ---------------------------
    // Main proposal loop
    // ---------------------------

    loop {
        let start = std::time::Instant::now();
        let timestamp = current_unix_timestamp();

        match engine.propose_block(proposer_id, &mut tx_pool, timestamp) {
            Ok((hash, block)) => {
                let elapsed = start.elapsed().as_secs_f64();
                metrics.consensus.block_validation_seconds.observe(elapsed);

                println!(
                    "proposed block height={} hash={}",
                    block.header.height,
                    hex::encode(hash.0.as_bytes()),
                );
            }
            Err(e) => {
                eprintln!("failed to propose block: {e}");
            }
        }

        tokio::time::sleep(Duration::from_secs(block_interval)).await;
    }
}

/// Returns the current wall-clock time as seconds since Unix epoch.
///
/// On error (system clock before epoch) this falls back to 0.
fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}
