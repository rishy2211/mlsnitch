//! Chain library crate.
//!
//! This crate provides the core building blocks for the prototype
//! consensus mechanism that "bakes in" ML authenticity checks:
//!
//! - strongly-typed domain types (`types`),
//! - a modular consensus engine (`consensus`),
//! - block validity predicates (`validation`),
//! - storage backends (`storage`),
//! - ML verification clients (`ml_client`),
//! - Prometheus-based metrics (`metrics`),
//! - and a top-level node configuration (`config`).
//!
//! Higher-level binaries can compose these pieces to build validator
//! nodes, simulators, and experiment harnesses.

pub mod config;
pub mod consensus;
pub mod metrics;
pub mod ml_client;
pub mod storage;
pub mod types;
pub mod validation;

// Re-export top-level configuration types.
pub use config::{ChainConfig, MetricsConfig, MlClientConfig};

// Re-export "core" consensus types and traits.
pub use consensus::{
    AcceptAllValidator, BlockStore, BlockValidator, CombinedValidator, ConsensusConfig,
    ConsensusEngine, ConsensusError, ForkChoice, LongestChainForkChoice, Proposer, TxPool,
    ValidationError,
};

// Re-export storage backends.
pub use storage::{InMemoryBlockStore, RocksDbBlockStore, RocksDbConfig, StorageError};

// Re-export ML verification interfaces and the HTTP client.
pub use ml_client::HttpMlVerifier;
pub use validation::{BaseValidity, MlConfig, MlError, MlValidity, MlVerifier};

// Re-export metrics registry and consensus metrics.
pub use metrics::{ConsensusMetrics, MetricsRegistry, run_prometheus_http_server};

// Re-export domain types at the crate root for convenience.
pub use types::*;

/// Type alias for the default block validator stack used by a "typical" node.
///
/// This composes:
///
/// - [`BaseValidity`] for cheap structural checks, and
/// - [`MlValidity<HttpMlVerifier>`] for ML authenticity checks.
pub type DefaultBlockValidator = CombinedValidator<BaseValidity, MlValidity<HttpMlVerifier>>;

/// Type alias for the default fork-choice rule.
pub type DefaultForkChoice = LongestChainForkChoice;

/// Type alias for the default block store backend.
pub type DefaultBlockStore = RocksDbBlockStore;

/// Type alias for the default consensus engine stack.
///
/// This uses:
///
/// - [`DefaultBlockStore`] (RocksDB),
/// - [`DefaultBlockValidator`] (base + ML),
/// - [`DefaultForkChoice`] (longest-chain-by-height).
pub type DefaultConsensusEngine =
    ConsensusEngine<DefaultBlockStore, DefaultBlockValidator, DefaultForkChoice>;
