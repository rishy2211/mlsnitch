//! HTTP-based ML verifier client.
//!
//! This implementation of [`crate::validation::MlVerifier`] talks to a
//! Python + PyTorch watermarking service over HTTP. It assumes the
//! service exposes a JSON API of the form:
//!
//! ```json
//! POST /verify
//! {
//!   "aid": "hex-encoded-aid",
//!   "scheme_id": "multi_factor_v1",
//!   "evidence_hash": "hex-encoded-evidence-hash",
//!   "wm_profile": {
//!     "tau_input": 0.9,
//!     "tau_feat": 0.1,
//!     "logit_band_low": 0.02,
//!     "logit_band_high": 0.05
//!   }
//! }
//!
//! Response:
//! {
//!   "ok": true,
//!   "trigger_acc": 0.94,
//!   "feat_dist": 0.07,
//!   "logit_stat": 0.031,
//!   "latency_ms": 123
//! }
//! ```
//!
//! The exact schema can be evolved alongside the Python service, as long
//! as it remains compatible with the request/response types defined here.

use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::types::{Aid, EvidenceHash, EvidenceRef, Hash256, WmProfile};
use crate::validation::{MlError, MlVerdict, MlVerifier};

/// HTTP-based ML verifier.
///
/// This client is thread-safe (`Send + Sync`) and can be shared across
/// validators. It uses the blocking `reqwest` client internally; higher
/// layers can wrap calls in dedicated threads or spawn blocking tasks
/// in a Tokio runtime if needed.
pub struct HttpMlVerifier {
    base_url: String,
    client: Client,
    timeout: Duration,
}

impl HttpMlVerifier {
    /// Constructs a new HTTP ML verifier pointing at `base_url`.
    ///
    /// `base_url` should be the root of the ML service, e.g.
    /// `"http://127.0.0.1:8080"` (without a trailing slash).
    pub fn new(base_url: impl Into<String>, timeout: Duration) -> Result<Self, MlError> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| MlError::Transport(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            base_url: base_url.into(),
            client,
            timeout,
        })
    }

    fn endpoint(&self, path: &str) -> String {
        // Avoid accidental double slashes.
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

/// Internal request payload sent to the ML service.
#[derive(Debug, Serialize)]
struct VerifyRequest {
    /// Hex-encoded model artefact identifier.
    aid: String,
    /// Watermark scheme identifier.
    scheme_id: String,
    /// Hex-encoded evidence hash.
    evidence_hash: String,
    /// Tuning profile for the watermark detector.
    wm_profile: WmProfile,
}

/// Internal response payload returned by the ML service.
#[derive(Debug, Deserialize)]
struct VerifyResponse {
    ok: bool,
    trigger_acc: Option<f32>,
    feat_dist: Option<f32>,
    logit_stat: Option<f32>,
    latency_ms: Option<u64>,
}

fn hash256_to_hex(h: &Hash256) -> String {
    hex::encode(h.as_bytes())
}

fn aid_to_hex(aid: &Aid) -> String {
    hash256_to_hex(aid.as_hash())
}

fn evidence_hash_to_hex(eh: &EvidenceHash) -> String {
    hash256_to_hex(eh.as_hash())
}

impl MlVerifier for HttpMlVerifier {
    fn verify(&self, aid: &Aid, evidence: &EvidenceRef) -> Result<MlVerdict, MlError> {
        let url = self.endpoint("/verify");

        let req_body = VerifyRequest {
            aid: aid_to_hex(aid),
            scheme_id: evidence.scheme_id.clone(),
            evidence_hash: evidence_hash_to_hex(&evidence.evidence_hash),
            wm_profile: evidence.wm_profile.clone(),
        };

        let resp = self
            .client
            .post(&url)
            .json(&req_body)
            .send()
            .map_err(|e| MlError::Transport(format!("HTTP POST {url} failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(MlError::Service(format!(
                "ML service returned HTTP status {status}"
            )));
        }

        let body = resp
            .json::<VerifyResponse>()
            .map_err(|e| MlError::Protocol(format!("failed to parse JSON response: {e}")))?;

        Ok(MlVerdict {
            ok: body.ok,
            trigger_acc: body.trigger_acc,
            feat_dist: body.feat_dist,
            logit_stat: body.logit_stat,
            latency_ms: body.latency_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EvidenceHash, HASH_LEN};

    #[test]
    fn verify_request_hex_encoding_helpers() {
        let h = Hash256([0xAB; HASH_LEN]);
        let eh = EvidenceHash(h);
        let aid = Aid(h);

        let aid_hex = aid_to_hex(&aid);
        let eh_hex = evidence_hash_to_hex(&eh);

        assert_eq!(aid_hex.len(), HASH_LEN * 2);
        assert_eq!(eh_hex.len(), HASH_LEN * 2);
        assert!(aid_hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(eh_hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_response_can_be_deserialized() {
        let json = r#"
        {
          "ok": true,
          "trigger_acc": 0.96,
          "feat_dist": 0.04,
          "logit_stat": 0.01,
          "latency_ms": 142
        }
        "#;

        let resp: VerifyResponse = serde_json::from_str(json).expect("VerifyResponse should parse");
        assert!(resp.ok);
        assert_eq!(resp.trigger_acc, Some(0.96));
        assert_eq!(resp.feat_dist, Some(0.04));
        assert_eq!(resp.logit_stat, Some(0.01));
        assert_eq!(resp.latency_ms, Some(142));
    }
}
