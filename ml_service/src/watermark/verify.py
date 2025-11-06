"""
Watermark verification logic.

For now this is deliberately lightweight and "stubby" so the end-to-end
system is runnable without a full watermark implementation:

- It loads the model (if present) via `torch.load` to sanity-check the
  artefact.
- It computes synthetic statistics deterministically from
  (aid, evidence_hash) so they are stable across runs.
- It compares those stats to the provided `WmProfile` to derive an `ok`
  verdict.

You can replace the synthetic parts with your real multi-factor
watermarking pipeline later.
"""

from __future__ import annotations

import hashlib
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Tuple

import torch

from ..schemas import WmProfile


@dataclass
class WatermarkStats:
    ok: bool
    trigger_acc: float
    feat_dist: float
    logit_stat: float
    latency_ms: int


def _pseudo_random_stats(
    aid_hex: str, evidence_hash_hex: str
) -> Tuple[float, float, float]:
    """
    Derive deterministic "pseudo-random" stats from (aid, evidence_hash).

    This avoids introducing actual randomness while still giving you
    non-trivial values for demos / tests.
    """
    seed_bytes = (aid_hex + evidence_hash_hex).encode("utf-8")
    digest = hashlib.blake2b(seed_bytes, digest_size=16).digest()

    # Split 16 bytes into three 5-byte-ish chunks and normalise:
    def to_unit_interval(b: bytes) -> float:
        return int.from_bytes(b, byteorder="big") / ((1 << (8 * len(b))) - 1)

    trigger_acc = 0.8 + 0.2 * to_unit_interval(digest[0:5])  # 0.8 .. 1.0
    feat_dist = 0.01 + 0.2 * to_unit_interval(digest[5:10])  # 0.01 .. 0.21
    logit_stat = -0.05 + 0.1 * to_unit_interval(digest[10:16])  # -0.05 .. 0.05

    return trigger_acc, feat_dist, logit_stat


def verify_model(
    model_path: Path,
    aid_hex: str,
    evidence_hash_hex: str,
    wm_profile: WmProfile,
) -> WatermarkStats:
    """
    Perform a lightweight watermark verification.

    For now:
    - if the model file cannot be loaded with `torch.load`, we treat it
      as `ok=False` with zeroed stats;
    - otherwise we derive deterministic pseudo-random stats from
      (aid, evidence_hash) and compare them to the provided thresholds.

    You can replace this body with a real multi-factor watermark
    detector later (trigger / feature / logit tests).
    """
    start = time.perf_counter()

    try:
        # This will fail fast if the file is missing or not a PyTorch artifact.
        _ = torch.load(model_path, map_location="cpu")
    except Exception:
        end = time.perf_counter()
        latency_ms = int((end - start) * 1000)
        return WatermarkStats(
            ok=False,
            trigger_acc=0.0,
            feat_dist=1.0,
            logit_stat=0.0,
            latency_ms=latency_ms,
        )

    trigger_acc, feat_dist, logit_stat = _pseudo_random_stats(
        aid_hex, evidence_hash_hex
    )

    # Multi-factor verdict:
    ok = (
        trigger_acc >= wm_profile.tau_input
        and feat_dist <= wm_profile.tau_feat
        and wm_profile.logit_band_low <= logit_stat <= wm_profile.logit_band_high
    )

    end = time.perf_counter()
    latency_ms = int((end - start) * 1000)

    return WatermarkStats(
        ok=ok,
        trigger_acc=trigger_acc,
        feat_dist=feat_dist,
        logit_stat=logit_stat,
        latency_ms=latency_ms,
    )
