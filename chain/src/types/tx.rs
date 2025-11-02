// chain/src/types/tx.rs

//! Transaction types for the consensus layer.
//!
//! This module defines the concrete transaction payloads used by the chain
//! along with a tagged [`Transaction`] enum. Transactions cover:
//!
//! - registering new ML model artefacts on-chain,
//! - recording usage events for existing models, and
//! - simple value transfers between accounts.

use serde::{Deserialize, Serialize};

use super::{AccountId, Aid, EvidenceRef, Signature};

/// Transaction that registers a new ML model artefact on-chain.
///
/// A `TxRegisterModel` is the only way to introduce a new model artefact
/// into the chain. Once accepted, the node updates state to include an
/// [`ArtefactMetadata`](crate::types::ArtefactMetadata) entry keyed by
/// the [`Aid`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxRegisterModel {
    /// Account that owns the model.
    ///
    /// This is the account that will be recorded as the initial owner of
    /// the artefact and typically the entity that can authorise transfers.
    pub owner: AccountId,

    /// Content-addressed artefact identifier for the model.
    ///
    /// The identifier must be computed from a canonical encoding of the
    /// model bytes (for example via [`Aid::from_model_bytes`]) so that the
    /// same logical artefact always maps to the same `Aid`.
    pub aid: Aid,

    /// Reference to authenticity / watermark evidence.
    ///
    /// This ties the registration to a particular watermarking scheme and
    /// parameter set. The evidence payload itself is stored off-chain and
    /// addressed via [`EvidenceRef`].
    pub evidence: EvidenceRef,

    /// Fee the owner is willing to pay for registration.
    ///
    /// The concrete fee semantics are determined by the execution layer
    /// (e.g. deducted from the owner’s balance).
    pub fee: u64,

    /// Anti-replay nonce relative to the owner account.
    ///
    /// Typically this is a monotonically increasing counter per account.
    /// Nodes reject transactions whose nonce is not strictly greater than
    /// the stored nonce for the signer.
    pub nonce: u64,

    /// Owner's signature over the canonical encoding of this transaction.
    ///
    /// The canonical encoding is defined by the transaction layer
    /// (e.g. bincode or a custom format). Verifiers must use the same
    /// encoding when checking signatures.
    pub signature: Signature,
}

/// Additional information about how a model is used.
///
/// `ModelUseMetadata` captures high-level semantics of a particular usage
/// event (e.g. task and logical version). It is intentionally small and
/// human-readable; heavy logs or telemetry should remain off-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelUseMetadata {
    /// Free-form description of the task (e.g. `"image_classification"`).
    pub task: String,

    /// Optional logical version of the model usage.
    ///
    /// This can distinguish different configurations, checkpoints, or
    /// deployment environments for the same [`Aid`].
    pub version: Option<String>,
}

/// Transaction that records use of an existing model.
///
/// A `TxUseModel` does **not** change model ownership. Instead it creates
/// an auditable record that a particular account invoked a model with
/// some semantics (described by [`ModelUseMetadata`]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxUseModel {
    /// Account invoking / using the model.
    ///
    /// This is the account that pays the fee and signs the transaction.
    pub caller: AccountId,

    /// Identifier of the model being used.
    ///
    /// Must refer to an existing, previously registered [`Aid`].
    pub aid: Aid,

    /// Usage metadata (task, version, etc.).
    pub metadata: ModelUseMetadata,

    /// Fee the caller is paying for this usage record.
    pub fee: u64,

    /// Anti-replay nonce relative to the caller account.
    pub nonce: u64,

    /// Caller’s signature over the canonical encoding.
    pub signature: Signature,
}

/// Optional simple value-transfer transaction.
///
/// `TxTransfer` is intentionally minimal: it moves a fungible balance from
/// one account to another while charging a fee. It can be used to prototype
/// fee markets or reward distribution without designing a full asset layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxTransfer {
    /// Account sending the funds.
    pub from: AccountId,

    /// Account receiving the funds.
    pub to: AccountId,

    /// Amount to transfer from `from` to `to`.
    pub amount: u64,

    /// Fee paid by `from` to include this transfer.
    pub fee: u64,

    /// Anti-replay nonce relative to the `from` account.
    pub nonce: u64,

    /// Signature by `from` over the canonical encoding of the transfer.
    pub signature: Signature,
}

/// Top-level transaction enum.
///
/// This is the type that appears in blocks and mempool structures. For
/// binary formats (bincode 2), we use the default externally-tagged
/// representation, which is supported by `bincode::serde`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Transaction {
    /// Registers a new ML model artefact on-chain.
    RegisterModel(TxRegisterModel),

    /// Records usage of an already-registered model.
    UseModel(TxUseModel),

    /// Simple fungible value transfer between accounts.
    Transfer(TxTransfer),
}

#[cfg(test)]
mod tests {
    use super::super::{EvidenceHash, HASH_LEN, Hash256, WmProfile};
    use super::*;

    fn dummy_hash(byte: u8) -> Hash256 {
        Hash256([byte; HASH_LEN])
    }

    fn dummy_wm_profile() -> WmProfile {
        WmProfile {
            tau_input: 0.1,
            tau_feat: 0.2,
            logit_band_low: -1.0,
            logit_band_high: 1.0,
        }
    }

    fn dummy_evidence(byte: u8) -> EvidenceRef {
        EvidenceRef {
            scheme_id: format!("wm-test-{byte}"),
            evidence_hash: EvidenceHash(dummy_hash(byte)),
            wm_profile: dummy_wm_profile(),
        }
    }

    fn dummy_signature() -> Signature {
        // Size is arbitrary here; it just needs to be non-empty.
        Signature(vec![7_u8; 64])
    }

    #[test]
    fn register_model_roundtrips_with_bincode2() {
        let owner = AccountId(dummy_hash(1));
        let aid = Aid(dummy_hash(2));
        let evidence = dummy_evidence(3);
        let signature = dummy_signature();

        let tx_reg = TxRegisterModel {
            owner,
            aid,
            evidence: EvidenceRef {
                scheme_id: evidence.scheme_id.clone(),
                evidence_hash: evidence.evidence_hash,
                wm_profile: dummy_wm_profile(),
            },
            fee: 42,
            nonce: 7,
            signature: Signature(signature.0.clone()),
        };

        let tx = Transaction::RegisterModel(tx_reg);

        let cfg = bincode::config::standard();
        let bytes =
            bincode::serde::encode_to_vec(&tx, cfg).expect("Transaction::RegisterModel encode");
        let (decoded, _): (Transaction, usize) = bincode::serde::decode_from_slice(&bytes, cfg)
            .expect("Transaction::RegisterModel decode");

        match decoded {
            Transaction::RegisterModel(decoded_tx) => {
                assert_eq!(decoded_tx.owner, owner);
                assert_eq!(decoded_tx.aid, aid);
                assert_eq!(decoded_tx.fee, 42);
                assert_eq!(decoded_tx.nonce, 7);
                assert_eq!(decoded_tx.signature.as_bytes(), signature.as_bytes());

                assert_eq!(decoded_tx.evidence.scheme_id, evidence.scheme_id);
                assert_eq!(
                    decoded_tx.evidence.evidence_hash.as_hash(),
                    evidence.evidence_hash.as_hash()
                );

                let p = decoded_tx.evidence.wm_profile;
                assert!((p.tau_input - 0.1).abs() < f32::EPSILON);
                assert!((p.tau_feat - 0.2).abs() < f32::EPSILON);
                assert!((p.logit_band_low + 1.0).abs() < f32::EPSILON);
                assert!((p.logit_band_high - 1.0).abs() < f32::EPSILON);
            }
            other => panic!("unexpected transaction variant: {other:?}"),
        }
    }

    #[test]
    fn use_model_roundtrips_with_bincode2() {
        let caller = AccountId(dummy_hash(4));
        let aid = Aid(dummy_hash(5));
        let metadata = ModelUseMetadata {
            task: "image_classification".to_string(),
            version: Some("v1".to_string()),
        };
        let signature = dummy_signature();

        let tx_use = TxUseModel {
            caller,
            aid,
            metadata: ModelUseMetadata {
                task: metadata.task.clone(),
                version: metadata.version.clone(),
            },
            fee: 10,
            nonce: 99,
            signature: Signature(signature.0.clone()),
        };

        let tx = Transaction::UseModel(tx_use);

        let cfg = bincode::config::standard();
        let bytes = bincode::serde::encode_to_vec(&tx, cfg).expect("Transaction::UseModel encode");
        let (decoded, _): (Transaction, usize) =
            bincode::serde::decode_from_slice(&bytes, cfg).expect("Transaction::UseModel decode");

        match decoded {
            Transaction::UseModel(decoded_tx) => {
                assert_eq!(decoded_tx.caller, caller);
                assert_eq!(decoded_tx.aid, aid);
                assert_eq!(decoded_tx.fee, 10);
                assert_eq!(decoded_tx.nonce, 99);
                assert_eq!(decoded_tx.signature.as_bytes(), signature.as_bytes());
                assert_eq!(decoded_tx.metadata.task, metadata.task);
                assert_eq!(decoded_tx.metadata.version, metadata.version);
            }
            other => panic!("unexpected transaction variant: {other:?}"),
        }
    }

    #[test]
    fn transfer_roundtrips_with_bincode2() {
        let from = AccountId(dummy_hash(6));
        let to = AccountId(dummy_hash(7));
        let signature = dummy_signature();

        let tx_transfer = TxTransfer {
            from,
            to,
            amount: 1_000,
            fee: 3,
            nonce: 5,
            signature: Signature(signature.0.clone()),
        };

        let tx = Transaction::Transfer(tx_transfer);

        let cfg = bincode::config::standard();
        let bytes = bincode::serde::encode_to_vec(&tx, cfg).expect("Transaction::Transfer encode");
        let (decoded, _): (Transaction, usize) =
            bincode::serde::decode_from_slice(&bytes, cfg).expect("Transaction::Transfer decode");

        match decoded {
            Transaction::Transfer(decoded_tx) => {
                assert_eq!(decoded_tx.from, from);
                assert_eq!(decoded_tx.to, to);
                assert_eq!(decoded_tx.amount, 1_000);
                assert_eq!(decoded_tx.fee, 3);
                assert_eq!(decoded_tx.nonce, 5);
                assert_eq!(decoded_tx.signature.as_bytes(), signature.as_bytes());
            }
            other => panic!("unexpected transaction variant: {other:?}"),
        }
    }
}
