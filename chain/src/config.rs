//! Top-level configuration for a chain node.
//!
//! This module aggregates configuration for:
//!
//! - consensus parameters (`ConsensusConfig`),
//! - storage (RocksDB path and creation flags),
//! - ML verification client (ML service URL + timeout),
//! - metrics exporter (enable flag + listen address).
//!
//! The goal is to have a single `ChainConfig` struct that higher-level
//! binaries (e.g. `main.rs`) can construct from defaults, config files,
//! or environment variables as needed.

use std::net::SocketAddr;
use std::time::Duration;

use crate::consensus::ConsensusConfig;
use crate::storage::RocksDbConfig;

/// Configuration for the ML verification client.
///
/// This is used to construct an HTTP or gRPC client that implements
/// `validation::MlVerifier`.
#[derive(Clone, Debug)]
pub struct MlClientConfig {
    /// Base URL of the ML verification service, e.g. `"http://127.0.0.1:8080"`.
    pub base_url: String,
    /// Request timeout for ML verification calls.
    pub timeout: Duration,
}

impl Default for MlClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080".to_string(),
            timeout: Duration::from_secs(2),
        }
    }
}

/// Configuration for the Prometheus metrics exporter.
#[derive(Clone, Debug)]
pub struct MetricsConfig {
    /// Whether to run a `/metrics` HTTP exporter.
    pub enabled: bool,
    /// Address to bind the metrics HTTP server to.
    pub listen_addr: SocketAddr,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        // Safe to unwrap: this is a fixed, valid address literal.
        let addr: SocketAddr = "127.0.0.1:9898"
            .parse()
            .expect("hard-coded metrics listen address should parse");
        Self {
            enabled: true,
            listen_addr: addr,
        }
    }
}

/// Top-level configuration for a chain node.
///
/// This aggregates all the sub-configs needed to wire up a typical node:
///
/// - consensus tuning (`consensus`),
/// - persistent storage (`storage`),
/// - ML verification client (`ml_client`),
/// - Prometheus metrics exporter (`metrics`).
#[derive(Clone, Debug, Default)]
pub struct ChainConfig {
    pub consensus: ConsensusConfig,
    pub storage: RocksDbConfig,
    pub ml_client: MlClientConfig,
    pub metrics: MetricsConfig,
}
