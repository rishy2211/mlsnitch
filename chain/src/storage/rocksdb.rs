//! RocksDB-backed block store.
//!
//! This implementation persists blocks and tip metadata in a RocksDB
//! instance with dedicated column families:
//!
//! - `"blocks"`: maps `BlockHash` (32 bytes) -> canonical block bytes,
//! - `"meta"`:   stores the current tip under a fixed key `"tip"`.

use std::{path::Path, sync::Arc};

use crate::consensus::store::BlockStore;
use crate::types::{Block, BlockHash, HASH_LEN, Hash256};

use rocksdb::{BoundColumnFamily, ColumnFamilyDescriptor, DB, Options};

/// Configuration for [`RocksDbBlockStore`].
#[derive(Clone, Debug)]
pub struct RocksDbConfig {
    /// Filesystem path to the RocksDB database directory.
    pub path: String,
    /// Whether to create the database and missing column families if they
    /// do not yet exist.
    pub create_if_missing: bool,
}

impl Default for RocksDbConfig {
    fn default() -> Self {
        Self {
            path: "data/chain-db".to_string(),
            create_if_missing: true,
        }
    }
}

/// Storage-level error type.
#[derive(Debug)]
pub enum StorageError {
    /// Underlying RocksDB error.
    RocksDb(rocksdb::Error),
    /// Required column family was not found.
    MissingColumnFamily(&'static str),
    /// Corrupted or malformed metadata (e.g. tip hash with wrong length).
    CorruptedMeta(&'static str),
}

impl From<rocksdb::Error> for StorageError {
    fn from(e: rocksdb::Error) -> Self {
        StorageError::RocksDb(e)
    }
}

/// RocksDB-backed implementation of [`BlockStore`].
pub struct RocksDbBlockStore {
    db: DB,
}

impl RocksDbBlockStore {
    /// Opens (or creates) a RocksDB-backed block store at the given path.
    ///
    /// This sets up the `"blocks"` and `"meta"` column families. The
    /// `"default"` column family is also created to keep RocksDB happy,
    /// but it is not currently used.
    pub fn open(cfg: &RocksDbConfig) -> Result<Self, StorageError> {
        let path = Path::new(&cfg.path);

        let mut opts = Options::default();
        opts.create_if_missing(cfg.create_if_missing);
        opts.create_missing_column_families(cfg.create_if_missing);

        let cfs = vec![
            ColumnFamilyDescriptor::new("default", Options::default()),
            ColumnFamilyDescriptor::new("blocks", Options::default()),
            ColumnFamilyDescriptor::new("meta", Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        Ok(Self { db })
    }

    fn cf_blocks(&self) -> Result<Arc<BoundColumnFamily<'_>>, StorageError> {
        self.db
            .cf_handle("blocks")
            .ok_or(StorageError::MissingColumnFamily("blocks"))
    }

    fn cf_meta(&self) -> Result<Arc<BoundColumnFamily<'_>>, StorageError> {
        self.db
            .cf_handle("meta")
            .ok_or(StorageError::MissingColumnFamily("meta"))
    }

    /// Internal helper: encodes a block into canonical bytes (bincode 2).
    fn encode_block(block: &Block) -> Vec<u8> {
        block.canonical_bytes()
    }

    /// Internal helper: decodes a block from canonical bytes.
    fn decode_block(bytes: &[u8]) -> Option<Block> {
        let cfg = bincode::config::standard();
        let (block, _): (Block, usize) = bincode::serde::decode_from_slice(bytes, cfg).ok()?;
        Some(block)
    }

    /// Loads the current tip hash from the meta column family, if present.
    fn load_tip(&self) -> Result<Option<BlockHash>, StorageError> {
        let cf_meta = self.cf_meta()?;
        match self.db.get_cf(&cf_meta, b"tip")? {
            None => Ok(None),
            Some(bytes) => {
                if bytes.len() != HASH_LEN {
                    return Err(StorageError::CorruptedMeta("tip hash length"));
                }
                let mut arr = [0u8; HASH_LEN];
                arr.copy_from_slice(&bytes);
                Ok(Some(BlockHash(Hash256(arr))))
            }
        }
    }

    /// Persists the tip hash into the meta column family.
    fn store_tip(&self, hash: &BlockHash) -> Result<(), StorageError> {
        let cf_meta = self.cf_meta()?;
        let bytes = hash.0.as_bytes();
        self.db.put_cf(&cf_meta, b"tip", bytes)?;
        Ok(())
    }
}

impl BlockStore for RocksDbBlockStore {
    fn get_block(&self, hash: &BlockHash) -> Option<Block> {
        let cf = self.cf_blocks().ok()?;
        let key = hash.0.as_bytes();
        match self.db.get_cf(&cf, key) {
            Ok(Some(bytes)) => Self::decode_block(&bytes),
            Ok(None) => None,
            Err(_) => None,
        }
    }

    fn put_block(&mut self, block: Block) {
        // We compute the hash before encoding so the mapping is consistent
        // with consensus-level hashing.
        let hash = block.compute_hash();
        let bytes = Self::encode_block(&block);

        if let Ok(cf) = self.cf_blocks() {
            // If this write fails, we log to stderr and drop the block.
            if let Err(e) = self.db.put_cf(&cf, hash.0.as_bytes(), bytes) {
                eprintln!("RocksDbBlockStore::put_block failed: {e}");
            }
        } else {
            eprintln!("RocksDbBlockStore::put_block: missing 'blocks' CF");
        }
    }

    fn tip(&self) -> Option<BlockHash> {
        self.load_tip().ok().flatten()
    }

    fn set_tip(&mut self, hash: BlockHash) {
        if let Err(e) = self.store_tip(&hash) {
            eprintln!("RocksDbBlockStore::set_tip failed: {e:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AccountId, Block, Header};
    use tempfile::TempDir;

    fn dummy_hash(byte: u8) -> Hash256 {
        Hash256([byte; HASH_LEN])
    }

    fn dummy_account(byte: u8) -> AccountId {
        AccountId(dummy_hash(byte))
    }

    fn dummy_block(height: u64) -> Block {
        let header = Header {
            parent: BlockHash(dummy_hash(0)),
            height,
            timestamp: 1_700_000_000 + height,
            proposer: dummy_account(1),
            pos_proof: None,
        };

        Block {
            header,
            txs: Vec::new(),
        }
    }

    #[test]
    fn rocksdb_store_roundtrip_block_and_tip() {
        let tmp = TempDir::new().expect("create temp dir");
        let cfg = RocksDbConfig {
            path: tmp.path().to_string_lossy().to_string(),
            create_if_missing: true,
        };

        let mut store = RocksDbBlockStore::open(&cfg).expect("open RocksDB");

        let block = dummy_block(0);
        let hash = block.compute_hash();
        store.put_block(block);

        let fetched = store.get_block(&hash).expect("block should exist");
        assert_eq!(fetched.header.height, 0);

        store.set_tip(hash);
        let tip = store.tip().expect("tip should be set");
        assert_eq!(tip.0.as_bytes(), hash.0.as_bytes());
    }
}
