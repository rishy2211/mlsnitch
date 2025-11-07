import os
from pathlib import Path

import torch
from fastapi.testclient import TestClient

from src.main import app
from src.registry.filesystem_store import FilesystemModelRegistry
from src.schemas import HealthResponse, WmProfile


def test_health_endpoint():
    client = TestClient(app)
    resp = client.get("/health")
    assert resp.status_code == 200

    data = resp.json()
    # Validate shape using the pydantic model.
    health = HealthResponse(**data)
    assert health.status == "ok"


def test_verify_endpoint_missing_model(tmp_path: Path, monkeypatch):
    # Point MODEL_ROOT to a temp dir to avoid cluttering real paths.
    monkeypatch.setenv("ML_SERVICE_MODEL_ROOT", str(tmp_path))

    # Replace app.state.registry with one rooted in tmp_path.
    registry = FilesystemModelRegistry(root=tmp_path)
    app.state.registry = registry

    client = TestClient(app)

    aid_hex = "abcd" * 16
    payload = {
        "aid": aid_hex,
        "scheme_id": "multi_factor_v1",
        "evidence_hash": "1234" * 16,
        "wm_profile": {
            "tau_input": 0.9,
            "tau_feat": 0.1,
            "logit_band_low": -0.05,
            "logit_band_high": 0.05,
        },
    }

    resp = client.post("/verify", json=payload)
    assert resp.status_code == 200

    data = resp.json()
    assert "ok" in data
    # With missing model, current implementation should return ok=False.
    assert data["ok"] is False


def test_verify_endpoint_with_valid_model(tmp_path: Path, monkeypatch):
    # Point MODEL_ROOT to a temp dir.
    monkeypatch.setenv("ML_SERVICE_MODEL_ROOT", str(tmp_path))

    # Replace app.state.registry with one rooted in tmp_path.
    registry = FilesystemModelRegistry(root=tmp_path)
    app.state.registry = registry

    client = TestClient(app)

    # Create a model file corresponding to `aid`.
    aid_hex = "face" * 16
    model_path = registry.resolve(aid_hex)
    os.makedirs(model_path.parent, exist_ok=True)
    torch.save({"hello": "world"}, model_path)

    wm_profile = WmProfile(
        tau_input=0.0,
        tau_feat=1.0,
        logit_band_low=-1.0,
        logit_band_high=1.0,
    )

    payload = {
        "aid": aid_hex,
        "scheme_id": "multi_factor_v1",
        "evidence_hash": "badd" * 16,
        "wm_profile": wm_profile.model_dump(),
    }

    resp = client.post("/verify", json=payload)
    assert resp.status_code == 200

    data = resp.json()
    # Check basic keys are present.
    for key in ("ok", "trigger_acc", "feat_dist", "logit_stat", "latency_ms"):
        assert key in data

    # Given lax thresholds, ok should be true.
    assert data["ok"] is True
