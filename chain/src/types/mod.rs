//! Core domain types used by the chain
//!
//! This module defines strongly-typed hashes, account indentifiers, model
//! artefact identifiers, and watermarking-related metadata that are shared
//! across the chain implementation. The goal is to avoid "naked" byte
//! buffers in public APIs and instead use domain-specific newtypes.

use serde::{Deserialize, Serialize};

/// Types related to ML artefacts stored and referenced on-chain.
pub mod artefact;

pub use artefact::ArtefactMetadata;

/// Length in bytes of all 256-bit hash types used in this module.
pub const HASH_LEN: usize = 32;

/// Strongly-typed 256-bit hash wrapper (BLAKE3-256).
///
/// This type is used as the backing representation for all fixed-size hashes
/// in the chain (account identifiers, artefact identifiers, watermark
/// evidence hashes, etc.). It is always exactly [`HASH_LEN`] bytes long.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Hash256(pub [u8; HASH_LEN]);

impl Hash256 {
    /// Computes a new [`Hash256`] as the BLAKE3-256 hash of `data`.
    ///
    /// The result is deterministic for a given byte slice and is suitable
    /// for use as an identifier or content hash, but it is **not**
    /// a password hash or KDF.
    pub fn compute(data: &[u8]) -> Self {
        let h = blake3::hash(data);
        Hash256(*h.as_bytes())
    }

    /// Returns the underlying 32-byte hash as a borrowed array.
    ///
    /// This is useful when interfacing with low-level APIs that expect a
    /// fixed-size byte array instead of a newtype wrapper.
    pub fn as_bytes(&self) -> &[u8; HASH_LEN] {
        &self.0
    }
}

/// Account identifier (hash of the Dilithium public key).
///
/// `AccountId` is derived from a Dilithium / ML-DSA public key using
/// [`Hash256::compute`]. This keeps account identifiers short and
/// opaque while preserving a stable mapping from public keys.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Hash256);

impl AccountId {
    /// Derives an [`AccountId`] from a Dilithium public key.
    ///
    /// The caller is responsible for passing the canonical byte encoding
    /// of the public key. Different encodings of the same key will result
    /// in different account identifiers.
    pub fn from_public_key(pk_bytes: &[u8]) -> Self {
        AccountId(Hash256::compute(pk_bytes))
    }

    /// Returns the underlying [`Hash256`] backing this account identifier.
    pub fn as_hash(&self) -> &Hash256 {
        &self.0
    }
}

/// Dilithium / ML-DSA public key bytes, wrapped to avoid naked `Vec<u8>`.
///
/// This type is intentionally opaque: it does not interpret or validate the
/// public key material, it only carries it through the API in a structured
/// way.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicKey(pub Vec<u8>);

impl PublicKey {
    /// Returns the raw public key bytes.
    ///
    /// The encoding is scheme-specific and must match whatever the signing
    /// implementation expects (e.g. `pqcrypto-mldsa` public key format).
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Dilithium / ML-DSA signature bytes, as produced by `pqcrypto-mldsa`.
///
/// These are detached signatures over a canonical transaction encoding.
/// The exact encoding is defined by higher-level transaction code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    /// Returns the raw signature bytes.
    ///
    /// The encoding is scheme-specific and must match whatever the verifying
    /// implementation expects (e.g. `pqcrypto-mldsa` signature format).
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Hash of watermark key and parameters.
///
/// `EvidenceHash` is an opaque handle to off-chain verification material
/// (e.g. watermark keys, thresholds, and other detector parameters) rather
/// than storing that material directly on-chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct EvidenceHash(pub Hash256);

impl EvidenceHash {
    /// Computes an [`EvidenceHash`] from an arbitrary byte slice.
    ///
    /// The caller is responsible for using a stable, canonical encoding for
    /// watermark-related parameters so that the same logical evidence always
    /// maps to the same hash.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        EvidenceHash(Hash256::compute(bytes))
    }

    /// Returns the underlying [`Hash256`] backing this evidence hash.
    pub fn as_hash(&self) -> &Hash256 {
        &self.0
    }
}

/// Model artefact identifier (`aid = BLAKE3(model_bytes)`).
///
/// `Aid` acts as a content-addressed identifier for ML model artefacts
/// (e.g. weight files, checkpoints). It is derived from the canonical
/// bytes of the artefact.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Aid(pub Hash256);

impl Aid {
    /// Derives an [`Aid`] from the canonical bytes of a model artefact.
    ///
    /// The caller must ensure that `model_bytes` is a canonical encoding
    /// (for example, a normalised archive format), otherwise logically
    /// equivalent models may receive different identifiers.
    pub fn from_model_bytes(model_bytes: &[u8]) -> Self {
        Aid(Hash256::compute(model_bytes))
    }

    /// Returns the underlying [`Hash256`] backing this artefact identifier.
    pub fn as_hash(&self) -> &Hash256 {
        &self.0
    }
}

/// High-level watermark profile used for verification and tuning.
///
/// These parameters describe how a particular watermarking configuration
/// probes a model (e.g. thresholds in input, feature, and logit space).
/// They are stored on-chain as part of [`EvidenceRef`] for auditability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WmProfile {
    /// Input-space threshold parameter for the watermark detector.
    pub tau_input: f32,
    /// Feature-space threshold parameter for the watermark detector.
    pub tau_feat: f32,
    /// Lower bound of the logit band probed for watermark evidence.
    pub logit_band_low: f32,
    /// Upper bound of the logit band probed for watermark evidence.
    pub logit_band_high: f32,
}

/// On-chain reference to off-chain watermark evidence and configuration.
///
/// An `EvidenceRef` ties together:
///
/// - a stable identifier for the watermarking scheme,
/// - a hash of the concrete watermark evidence (keys and parameters),
/// - and a human-readable [`WmProfile`] describing how the scheme is tuned.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Stable identifier of the watermarking scheme (e.g. `"wm-laplace-v1"`).
    pub scheme_id: String,
    /// Hash of the opaque watermark evidence payload.
    pub evidence_hash: EvidenceHash,
    /// Watermark profile describing thresholds and bands used by the scheme.
    pub wm_profile: WmProfile,
}
