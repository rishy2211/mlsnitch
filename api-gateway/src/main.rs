// api-gateway/src/main.rs

//! API gateway binary.
//!
//! This binary exposes a small HTTP API on top of the `chain` crate:
//!
//! - `GET /health`
//! - `POST /models/register`
//!
//! It embeds a `DefaultConsensusEngine` (RocksDB-backed), a simple queued
//! transaction pool, a background block producer loop, and a Prometheus
//! metrics exporter on `/metrics`.

mod config;
mod routes;
mod state;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::signal;

use chain::{
    AccountId, BaseValidity, ChainConfig, CombinedValidator, Hash256, HttpMlVerifier,
    MetricsRegistry, MlConfig, MlValidity, run_prometheus_http_server,
};
use config::ApiConfig;
use routes::{health, models};
use state::{AppState, QueuedTxPool, SharedState};

#[tokio::main]
async fn main() {
    // Basic tracing setup.
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "api_gateway=info,chain=info".to_string()),
        )
        .init();

    if let Err(e) = run().await {
        eprintln!("fatal error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    // For now we use default configs. These can be externalised later.
    let api_cfg = ApiConfig::default();
    let chain_cfg = ChainConfig::default();

    // ---------------------------
    // Metrics
    // ---------------------------

    let metrics = Arc::new(
        MetricsRegistry::new()
            .map_err(|e| format!("failed to initialise metrics registry: {e}"))?,
    );

    // Metrics exporter.
    if chain_cfg.metrics.enabled {
        let metrics_clone = metrics.clone();
        let addr = chain_cfg.metrics.listen_addr;
        tokio::spawn(async move {
            if let Err(e) = run_prometheus_http_server(metrics_clone, addr).await {
                eprintln!("metrics HTTP server error: {e}");
            }
        });
        tracing::info!("metrics exporter listening on http://{}/metrics", addr);
    }

    // ---------------------------
    // Storage + consensus engine
    // ---------------------------

    let store = chain::RocksDbBlockStore::open(&chain_cfg.storage).map_err(|e| {
        format!(
            "failed to open RocksDB store at {}: {e:?}",
            chain_cfg.storage.path
        )
    })?;

    let ml_verifier = HttpMlVerifier::new(
        chain_cfg.ml_client.base_url.clone(),
        chain_cfg.ml_client.timeout,
    )
    .map_err(|e| format!("failed to create HttpMlVerifier: {e:?}"))?;

    let base_validity = BaseValidity::new(&chain_cfg.consensus);
    let ml_validity = MlValidity::new(ml_verifier, MlConfig::default());
    let validator = CombinedValidator::new(base_validity, ml_validity);

    let fork_choice = chain::DefaultForkChoice::default();

    let engine: chain::DefaultConsensusEngine =
        chain::ConsensusEngine::new(chain_cfg.consensus.clone(), store, validator, fork_choice);

    // ---------------------------
    // Proposer identity + tx pool
    // ---------------------------

    // In a real node this would be derived from a Dilithium public key.
    let proposer_id = {
        let seed = b"api-gateway-proposer";
        AccountId(Hash256::compute(seed))
    };

    let tx_pool = QueuedTxPool::new();

    // ---------------------------
    // Shared state
    // ---------------------------

    let app_state: SharedState = Arc::new(AppState {
        engine: tokio::sync::Mutex::new(engine),
        tx_pool: tokio::sync::Mutex::new(tx_pool),
        proposer_id,
        metrics: metrics.clone(),
    });

    // ---------------------------
    // Block producer loop
    // ---------------------------

    let block_interval_secs = chain_cfg.consensus.block_time_secs;
    let producer_state = app_state.clone();
    tokio::spawn(async move {
        run_block_producer(producer_state, block_interval_secs).await;
    });

    // ---------------------------
    // HTTP router
    // ---------------------------

    let app = Router::new()
        .route("/health", get(health::health))
        .route("/models/register", post(models::register_model))
        .with_state(app_state);

    // ---------------------------
    // axum 0.8 server (hyper 1 / tokio 1.48 style)
    // ---------------------------

    tracing::info!("API gateway listening on http://{}", api_cfg.listen_addr);

    let listener = tokio::net::TcpListener::bind(api_cfg.listen_addr)
        .await
        .map_err(|e| format!("failed to bind {}: {e}", api_cfg.listen_addr))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| format!("API server error: {e}"))?;

    Ok(())
}

/// Background block producer loop.
///
/// Periodically asks the consensus engine to propose and import a new block
/// using the queued transaction pool.
async fn run_block_producer(state: SharedState, interval_secs: u64) {
    let interval = std::time::Duration::from_secs(interval_secs.max(1));
    tracing::info!(
        "block producer running with interval {}s",
        interval.as_secs()
    );

    loop {
        let start = std::time::Instant::now();
        let timestamp = current_unix_timestamp();

        {
            let mut engine_guard = state.engine.lock().await;
            let mut pool_guard = state.tx_pool.lock().await;

            match engine_guard.propose_block(state.proposer_id, &mut *pool_guard, timestamp) {
                Ok((hash, block)) => {
                    let elapsed = start.elapsed().as_secs_f64();
                    state
                        .metrics
                        .consensus
                        .block_validation_seconds
                        .observe(elapsed);

                    tracing::info!(
                        height = block.header.height,
                        hash = %hex::encode(hash.0.as_bytes()),
                        "proposed block"
                    );
                }
                Err(e) => {
                    tracing::warn!("failed to propose block: {e}");
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// Returns the current wall-clock time as seconds since Unix epoch.
fn current_unix_timestamp() -> u64 {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

/// Waits for Ctrl-C and returns, used for graceful shutdown.
async fn shutdown_signal() {
    // Wait for Ctrl+C
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
