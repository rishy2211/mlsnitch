"""
FastAPI app for the ML authenticity verification service.

Endpoints:
- GET /health
- POST /verify
"""

from __future__ import annotations

from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from .config import MODEL_ROOT
from .registry.filesystem_store import FilesystemModelRegistry
from .schemas import HealthResponse, VerifyRequest, VerifyResponse
from .watermark.verify import verify_model

app = FastAPI(
    title="ML Authenticity Verification Service",
    version="0.1.0",
    description="Watermark-based authenticity checks for ML artefacts used by the chain prototype.",
)

# Permissive CORS for dev; tighten this later if needed.
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# Attach the model registry to app state for reuse.
app.state.registry = FilesystemModelRegistry(MODEL_ROOT)


@app.get("/health", response_model=HealthResponse)
async def health() -> HealthResponse:
    """Simple health-check endpoint."""
    return HealthResponse(status="ok")


@app.post("/verify", response_model=VerifyResponse)
async def verify(req: VerifyRequest) -> VerifyResponse:
    """
    Verify authenticity of a model artefact.

    This endpoint is called by the Rust `HttpMlVerifier` client in the
    `chain` crate. It expects the `VerifyRequest` / `VerifyResponse`
    shapes defined in `schemas.py`.
    """
    registry: FilesystemModelRegistry = app.state.registry
    model_path: Path = registry.resolve(req.aid)

    # We *do not* return 404 if the model is missing; instead the verifier
    # returns `ok=False` so the consensus layer treats it as an authenticity
    # failure rather than a transport error.
    stats = verify_model(
        model_path=model_path,
        aid_hex=req.aid,
        evidence_hash_hex=req.evidence_hash,
        wm_profile=req.wm_profile,
    )

    return VerifyResponse(
        ok=stats.ok,
        trigger_acc=stats.trigger_acc,
        feat_dist=stats.feat_dist,
        logit_stat=stats.logit_stat,
        latency_ms=stats.latency_ms,
    )


def run() -> None:
    """
    Convenience entrypoint if you want to run via:

        python -m src.main

    or via the `ml-service` console_script defined in pyproject.toml.
    """
    import uvicorn

    uvicorn.run(
        "src.main:app",
        host="0.0.0.0",
        port=8080,
        reload=False,
    )
