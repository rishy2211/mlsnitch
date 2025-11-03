//! Consensus engine and related abstractions.
//!
//! This module provides a modular, testable consensus layer consisting of:
//!
//! - configuration parameters ([`config::ConsensusConfig`]),

pub mod config;
pub mod error;
pub mod fork_choice;
pub mod proposer;
pub mod store;
pub mod validator;

pub use config::ConsensusConfig;
pub use error::{ConsensusError, ValidationError};
pub use fork_choice::{ForkChoice, LongestChainForkChoice};
pub use proposer::{Proposer, TxPool};
pub use store::BlockStore;
pub use validator::{AcceptAllValidator, BlockValidator, CombinedValidator};
