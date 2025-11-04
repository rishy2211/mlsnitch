//! Metrics and instrumentation for the chain.
//!
//! This module defines Prometheus-compatible metrics for the consensus
//! engine and exposes a small HTTP exporter that serves `/metrics` in
//! Prometheus text format.
//!
//! Typical usage in a node:
//!
//! ```ignore
//! use std::net::SocketAddr;
//! use std::sync::Arc;
//! use chain::metrics::{MetricsRegistry, run_prometheus_http_server};
//!
//! let registry = Arc::new(MetricsRegistry::new()?);
//! let addr: SocketAddr = "127.0.0.1:9898".parse()?;
//!
//! // Spawn the HTTP exporter in the background:
//! tokio::spawn(run_prometheus_http_server(registry.clone(), addr));
//!
//! // Elsewhere in the code:
//! registry.consensus.block_validation_seconds.observe(duration_secs);
//! ```

pub mod prometheus;

pub use prometheus::{ConsensusMetrics, MetricsRegistry, run_prometheus_http_server};
