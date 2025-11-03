//! Consensus engine and related abstractions.
//!
//! This module provides a modular, testable consensus layer consisting of:
//!
//! - configuration parameters ([`config::ConsensusConfig`]),

pub mod config;

pub use config::ConsensusConfig;
