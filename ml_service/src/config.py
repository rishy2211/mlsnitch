"""
Configuration helpers for the ML service.

Right now this is intentionally minimal: we just define where models
(i.e. artefact weight files) live on disk. You can extend this later to
use Pydantic settings / env vars / TOML config.
"""

from __future__ import annotations

import os
from pathlib import Path

# Root directory for stored models.
# Each model is expected to live at `<MODEL_ROOT>/<aid>.pt` where `aid`
# is the hex-encoded artefact identifier used on-chain.
MODEL_ROOT: Path = Path(os.environ.get("ML_SERVICE_MODEL_ROOT", "models")).resolve()
