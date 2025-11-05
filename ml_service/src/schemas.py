"""
Pydantic schemas used by the FastAPI app.

These schemas mirror the JSON interface expected by the Rust
`HttpMlVerifier` in the `chain` crate.
"""

from __future__ import annotations

from typing import Optional

from pydantic import BaseModel


class WmProfile(BaseModel):
    """
    Watermark profile as passed from the chain.

    Matches the Rust `WmProfile` struct:

    - tau_input
    - tau_feat
    - logit_band_low
    - logit_band_high
    """

    tau_input: float
    tau_feat: float
    logit_band_low: float
    logit_band_high: float


class VerifyRequest(BaseModel):
    """
    Request payload for POST /verify.

    Fields are chosen to match the Rust `VerifyRequest` used by
    `HttpMlVerifier`:

    - aid: hex-encoded artefact ID (Aid)
    - scheme_id: watermark scheme identifier
    - evidence_hash: hex-encoded hash of the watermark key/params
    - wm_profile: thresholds and bands for verification
    """

    aid: str
    scheme_id: str
    evidence_hash: str
    wm_profile: WmProfile


class VerifyResponse(BaseModel):
    """
    Response payload for POST /verify.

    This mirrors the Rust `VerifyResponse` struct:

    - ok: overall authenticity verdict
    - trigger_acc: optional trigger accuracy statistic
    - feat_dist: optional feature-space distance statistic
    - logit_stat: optional logit-space statistic
    - latency_ms: optional end-to-end verification latency in milliseconds
    """

    ok: bool
    trigger_acc: Optional[float] = None
    feat_dist: Optional[float] = None
    logit_stat: Optional[float] = None
    latency_ms: Optional[int] = None


class HealthResponse(BaseModel):
    """Simple health check response."""

    status: str
