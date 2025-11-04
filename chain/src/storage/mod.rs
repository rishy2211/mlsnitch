//! Storage backends for the chain.
//!
//! This module provides concrete implementations of the
//! [`crate::consensus::store::BlockStore`] trait, including:
//!
//! - an in-memory store ([`mem::InMemoryBlockStore`]) suitable for tests,
//! - a RocksDB-backed store ([`rocksdb::RocksDbBlockStore`]) for persistent
//!   validator nodes.

pub mod mem;
pub mod rocksdb;

pub use mem::InMemoryBlockStore;
