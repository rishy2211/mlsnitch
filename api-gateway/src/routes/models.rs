use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use chain::{
    AccountId, Aid, EvidenceHash, EvidenceRef, HASH_LEN, Hash256, Signature, Transaction, WmProfile,
};

use crate::state::SharedState;

/// Request body for `POST /models/register`.
///
/// This is intentionally minimal: the client passes
/// - `owner_account_hex`: hex-encoded `AccountId` (Hash256),
/// - `aid_hex`: hex-encoded `Aid` (Hash256),
/// - `scheme_id`, `evidence_hash_hex`, and `wm_profile` parameters.
#[derive(Debug, Deserialize)]
pub struct RegisterModelRequest {
    /// Hex-encoded account identifier for the model owner.
    pub owner_account_hex: String,
    /// Hex-encoded model artefact identifier (`Aid`).
    pub aid_hex: String,
    /// Watermark scheme identifier.
    pub scheme_id: String,
    /// Hex-encoded evidence hash (hash of watermark key + parameters).
    pub evidence_hash_hex: String,
    /// Watermark profile thresholds and bands.
    pub wm_profile: WmProfileDto,
}

/// DTO version of [`WmProfile`] used in the API.
#[derive(Debug, Deserialize)]
pub struct WmProfileDto {
    pub tau_input: f32,
    pub tau_feat: f32,
    pub logit_band_low: f32,
    pub logit_band_high: f32,
}

impl From<WmProfileDto> for WmProfile {
    fn from(dto: WmProfileDto) -> Self {
        WmProfile {
            tau_input: dto.tau_input,
            tau_feat: dto.tau_feat,
            logit_band_low: dto.logit_band_low,
            logit_band_high: dto.logit_band_high,
        }
    }
}

/// Response body for `POST /models/register`.
#[derive(Debug, Serialize)]
pub struct RegisterModelResponse {
    pub status: &'static str,
    pub aid: String,
}

/// Parses a 32-byte hex string into a `Hash256`.
fn hex_to_hash256(hex_str: &str) -> Result<Hash256, &'static str> {
    let bytes = hex::decode(hex_str).map_err(|_| "invalid hex encoding")?;
    if bytes.len() != HASH_LEN {
        return Err("expected 32-byte hash");
    }
    let mut arr = [0u8; HASH_LEN];
    arr.copy_from_slice(&bytes);
    Ok(Hash256(arr))
}

/// `POST /models/register`
///
/// Queues a `TxRegisterModel` into the local transaction pool. The block
/// producer loop will eventually include it in a block, subject to
/// validity predicates.
pub async fn register_model(
    State(state): State<SharedState>,
    Json(body): Json<RegisterModelRequest>,
) -> Result<(StatusCode, Json<RegisterModelResponse>), (StatusCode, String)> {
    // Parse owner account.
    let owner_hash = hex_to_hash256(&body.owner_account_hex).map_err(as_bad_request)?;
    let owner = AccountId(owner_hash);

    // Parse aid.
    let aid_hash = hex_to_hash256(&body.aid_hex).map_err(as_bad_request)?;
    let aid = Aid(aid_hash);

    // Parse evidence hash.
    let evidence_hash = hex_to_hash256(&body.evidence_hash_hex).map_err(as_bad_request)?;
    let ev_hash = EvidenceHash(evidence_hash);

    let wm_profile: WmProfile = body.wm_profile.into();
    let evidence = EvidenceRef {
        scheme_id: body.scheme_id.clone(),
        evidence_hash: ev_hash,
        wm_profile,
    };

    // In a full implementation the client would sign the canonical
    // transaction encoding with a Dilithium key. For now we accept an
    // empty signature placeholder.
    let tx_reg = chain::TxRegisterModel {
        owner,
        aid,
        evidence,
        fee: 0,
        nonce: 0,
        signature: Signature(Vec::new()),
    };

    let tx = Transaction::RegisterModel(tx_reg);

    {
        // Enqueue the transaction.
        let mut pool = state.tx_pool.lock().await;
        pool.push(tx);
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(RegisterModelResponse {
            status: "queued",
            aid: body.aid_hex,
        }),
    ))
}

fn as_bad_request(msg: &'static str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}
