//! Prometheus-backed metrics and HTTP exporter.
//!
//! This module defines a [`MetricsRegistry`] that owns a Prometheus
//! registry and a set of strongly-typed consensus metrics, and an
//! async HTTP exporter that serves `/metrics` using `hyper`.

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode, body::Incoming, header, server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use prometheus::{
    self, Encoder, Histogram, HistogramOpts, IntCounter, Opts, Registry, TextEncoder,
};

/// Consensus-related Prometheus metrics.
///
/// These are registered into a [`Registry`] and can be updated from
/// consensus / validation code.
#[derive(Clone)]
pub struct ConsensusMetrics {
    /// Latency of full block validation (base + ML), in seconds.
    pub block_validation_seconds: Histogram,
    /// Time spent in ML authenticity checks (`V_auth`), in seconds.
    pub ml_auth_seconds: Histogram,
    /// Ratio of ML cache hits over total ML lookups (0–1).
    ///
    /// This is intended to be updated periodically by whatever component
    /// manages the ML verdict cache.
    pub ml_cache_hit_ratio: prometheus::Gauge,
    /// Number of blocks rejected due to ML authenticity failures.
    pub blocks_rejected_ml: IntCounter,
}

impl ConsensusMetrics {
    /// Registers consensus metrics into the given `Registry`.
    pub fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        // Block validation latency.
        let block_validation_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_block_validation_seconds",
                "Time to validate a block (base + ML) in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
        )?;
        registry.register(Box::new(block_validation_seconds.clone()))?;

        // ML authenticity latency.
        let ml_auth_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "consensus_ml_auth_seconds",
                "Time spent in ML authenticity checks (V_auth) per block in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
        )?;
        registry.register(Box::new(ml_auth_seconds.clone()))?;

        // ML cache hit ratio.
        let ml_cache_hit_ratio = prometheus::Gauge::with_opts(Opts::new(
            "consensus_ml_cache_hit_ratio",
            "Ratio of ML cache hits over total ML lookups (0..1)",
        ))?;
        registry.register(Box::new(ml_cache_hit_ratio.clone()))?;

        // Blocks rejected due to ML authenticity failures.
        let blocks_rejected_ml = IntCounter::with_opts(Opts::new(
            "consensus_blocks_rejected_ml",
            "Total number of blocks rejected due to ML authenticity failures",
        ))?;
        registry.register(Box::new(blocks_rejected_ml.clone()))?;

        Ok(Self {
            block_validation_seconds,
            ml_auth_seconds,
            ml_cache_hit_ratio,
            blocks_rejected_ml,
        })
    }
}

/// Wrapper around a Prometheus registry and the consensus metrics.
///
/// This is the main handle you pass around in the node. It can be wrapped
/// in an [`Arc`] and shared across threads/tasks.
#[derive(Clone)]
pub struct MetricsRegistry {
    registry: Registry,
    pub consensus: ConsensusMetrics,
}

impl MetricsRegistry {
    /// Creates a new `MetricsRegistry` with a fresh underlying `Registry`
    /// and registers the consensus metrics.
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new_custom(Some("chain".to_string()), None)?;
        let consensus = ConsensusMetrics::register(&registry)?;
        Ok(Self {
            registry,
            consensus,
        })
    }

    /// Encodes all metrics in this registry into the Prometheus text format.
    pub fn gather_text(&self) -> String {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
            eprintln!("failed to encode Prometheus metrics: {e}");
            return String::new();
        }
        String::from_utf8(buffer).unwrap_or_default()
    }
}

/// Runs an HTTP server that exposes Prometheus metrics.
///
/// The server listens on `addr` and serves `GET /metrics` with the
/// Prometheus text exposition format. All other paths return 404.
///
/// This function is `async` and is intended to be spawned onto a Tokio
/// runtime, e.g.:
///
/// ```ignore
/// let registry = Arc::new(MetricsRegistry::new()?);
/// let addr: SocketAddr = "127.0.0.1:9898".parse()?;
/// tokio::spawn(run_prometheus_http_server(registry.clone(), addr));
/// ```
pub async fn run_prometheus_http_server(
    metrics: Arc<MetricsRegistry>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let metrics = metrics.clone();

        tokio::spawn(async move {
            let svc = service_fn(move |req| {
                let metrics = metrics.clone();
                handle_request(req, metrics)
            });

            if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("prometheus HTTP server error: {err}");
            }
        });
    }
}

async fn handle_request(
    req: Request<Incoming>,
    metrics: Arc<MetricsRegistry>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let body = metrics.gather_text();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("not found")))
            .unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::Registry;

    #[test]
    fn consensus_metrics_register_and_record() {
        let registry = Registry::new();
        let metrics = ConsensusMetrics::register(&registry).expect("register metrics");

        metrics.block_validation_seconds.observe(0.123);
        metrics.ml_auth_seconds.observe(0.045);
        metrics.ml_cache_hit_ratio.set(0.75);
        metrics.blocks_rejected_ml.inc();

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn metrics_registry_gather_text_works() {
        let registry = MetricsRegistry::new().expect("create metrics registry");
        registry.consensus.block_validation_seconds.observe(0.01);
        let text = registry.gather_text();
        assert!(text.contains("consensus_block_validation_seconds"));
    }
}
