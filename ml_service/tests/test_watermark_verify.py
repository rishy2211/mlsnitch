from pathlib import Path

import torch

from src.schemas import WmProfile
from src.watermark.verify import verify_model


def test_verify_model_missing_file_returns_not_ok(tmp_path: Path):
    # Model path does not exist.
    model_path = tmp_path / "missing.pt"

    wm_profile = WmProfile(
        tau_input=0.9,
        tau_feat=0.1,
        logit_band_low=-0.05,
        logit_band_high=0.05,
    )

    stats = verify_model(
        model_path=model_path,
        aid_hex="abcd" * 16,
        evidence_hash_hex="1234" * 16,
        wm_profile=wm_profile,
    )

    assert stats.ok is False
    # Latency should be non-negative.
    assert stats.latency_ms >= 0


def test_verify_model_with_valid_torch_file(tmp_path: Path):
    # Save a simple PyTorch object to disk.
    model_path = tmp_path / "model.pt"
    torch.save({"foo": "bar"}, model_path)

    # Choose thresholds that are extremely lenient so `ok` should always be True
    # given the ranges in `_pseudo_random_stats`.
    wm_profile = WmProfile(
        tau_input=0.0,  # trigger_acc in [0.8, 1.0]
        tau_feat=1.0,  # feat_dist in [0.01, 0.21]
        logit_band_low=-1.0,  # logit_stat in [-0.05, 0.05]
        logit_band_high=1.0,
    )

    stats = verify_model(
        model_path=model_path,
        aid_hex="abcd" * 16,
        evidence_hash_hex="1234" * 16,
        wm_profile=wm_profile,
    )

    assert stats.ok is True
    assert 0.8 <= stats.trigger_acc <= 1.0
    assert 0.01 <= stats.feat_dist <= 0.21
    assert -0.05 <= stats.logit_stat <= 0.05
    assert stats.latency_ms >= 0
