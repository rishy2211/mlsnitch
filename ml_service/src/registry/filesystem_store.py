"""
Simple filesystem-backed model registry.

Each model artefact is expected to live at:

    <MODEL_ROOT>/<aid_hex>.pt

where `aid_hex` is the lower-case hex string passed in the /verify
request.
"""

from __future__ import annotations

from pathlib import Path
from typing import Optional

from ..config import MODEL_ROOT


class FilesystemModelRegistry:
    """Resolves model artefact IDs to local filesystem paths."""

    def __init__(self, root: Optional[Path] = None) -> None:
        self._root = root or MODEL_ROOT

    @property
    def root(self) -> Path:
        return self._root

    def resolve(self, aid_hex: str) -> Path:
        """
        Resolve a hex-encoded `aid` into a model path.

        No validation is done on the path beyond joining it to the root.
        """
        # Normalise to lower-case, strip any 0x prefix just in case.
        safe_aid = aid_hex.lower().removeprefix("0x")
        return self._root / f"{safe_aid}.pt"
