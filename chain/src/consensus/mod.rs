//! Consensus engine and related abstractions.
//!
//! This module provides a modular, testable consensus layer consisting of:
//!
//! - configuration parameters ([`config::ConsensusConfig`]),

pub mod config;
pub mod error;
pub mod store;

pub use config::ConsensusConfig;
pub use error::{ConsensusError, ValidationError};
pub use store::BlockStore;
