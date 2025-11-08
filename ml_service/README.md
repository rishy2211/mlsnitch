# ml_service – ML Authenticity Verification Service

This is the Python **FastAPI** service that the Rust `chain` crate talks to
for **watermark-based ML authenticity checks**.

It exposes a small HTTP API:

- `GET /health` – liveness check
- `POST /verify` – verify a model artefact (by `aid`) against watermark
  evidence and thresholds

The service is intentionally lightweight: the current implementation uses a
stubbed multi-factor watermark verifier that:

- loads a PyTorch model from disk (sanity check),
- derives deterministic pseudo-random statistics from `(aid, evidence_hash)`,
- compares them to the provided thresholds (`WmProfile`) to produce an `ok`
  verdict.

You can later replace the stub with your full multi-factor watermark logic
(trigger / feature / logit).

---

## API

### `GET /health`

Simple liveness check.

**Response:**

```json
{
  "status": "ok"
}
```

````

---

### `POST /verify`

Verify authenticity of a model artefact.

**Request body** (matches the Rust `HttpMlVerifier` client):

```json
{
  "aid": "hex-encoded-aid",
  "scheme_id": "multi_factor_v1",
  "evidence_hash": "hex-encoded-evidence-hash",
  "wm_profile": {
    "tau_input": 0.9,
    "tau_feat": 0.1,
    "logit_band_low": -0.05,
    "logit_band_high": 0.05
  }
}
```

- `aid` – hex-encoded `Aid` (BLAKE3-256 artefact ID used on-chain)
- `scheme_id` – watermark scheme identifier (`EvidenceRef.scheme_id`)
- `evidence_hash` – hex-encoded `EvidenceHash` (BLAKE3-256 of watermark key+params)
- `wm_profile` – thresholds used by the detector (`WmProfile`)

**Response body**:

```json
{
  "ok": true,
  "trigger_acc": 0.96,
  "feat_dist": 0.04,
  "logit_stat": 0.01,
  "latency_ms": 142
}
```

Fields:

- `ok` – overall verdict (what the Rust `MlValidity` uses)
- `trigger_acc` – synthetic trigger accuracy statistic
- `feat_dist` – synthetic feature-space distance
- `logit_stat` – synthetic logit-space statistic
- `latency_ms` – time spent in verification

If the model file cannot be loaded with `torch.load`, the service returns
`ok: false` and dummy stats, so the chain treats it as an authenticity
failure (not a transport error).

---

## Model Layout

The service uses a simple filesystem registry:

```text
<MODEL_ROOT>/<aid_hex>.pt
```

- `MODEL_ROOT` is read from the env var `ML_SERVICE_MODEL_ROOT`.
- If unset, it defaults to `models` under the current working directory.

Example:

- `ML_SERVICE_MODEL_ROOT=/app/ml_service/models`
- `aid = "aaaaaaaa...aaaa"` (64 hex chars)
- expected path: `/app/ml_service/models/aaaaaaaa...aaaa.pt`

The mapping is implemented in `src/registry/filesystem_store.py`.

---

## Code Layout

All code lives directly under `src/` (no nested `ml_service/ml_service`):

```text
src/
  __init__.py
  main.py          # FastAPI app + /health and /verify
  config.py        # MODEL_ROOT (via env), basic settings
  schemas.py       # Pydantic models: WmProfile, VerifyRequest, VerifyResponse

  registry/
    __init__.py
    filesystem_store.py  # aid_hex -> model path

  watermark/
    __init__.py
    verify.py      # stubbed watermark verification + stats

  models/
    __init__.py
    resnet.py      # SmallResNet example architecture (optional)
```

The FastAPI app is defined in `src/main.py` as `app`.

---

## Local Development

### 1. Create a virtualenv and install

```bash
cd ml_service

python -m venv .venv
source .venv/bin/activate

pip install -e .
```

This uses `pyproject.toml`:

```toml
[project]
name = "ml-service"
dependencies = [
  "fastapi",
  "uvicorn[standard]",
  "pydantic",
  "torch",
  "numpy",
]
```

### 2. Prepare a dummy model

For quick testing you can just drop an empty file:

```bash
mkdir -p models
touch models/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.pt
```

### 3. Run the service

```bash
uvicorn src.main:app --host 0.0.0.0 --port 8080
```

Or, using the script defined in `pyproject.toml`:

```bash
ml-service  # if your PATH picks up the console script
```

### 4. Test with curl

```bash
curl -X POST http://127.0.0.1:8080/verify \
  -H "Content-Type: application/json" \
  -d '{
    "aid": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "scheme_id": "multi_factor_v1",
    "evidence_hash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "wm_profile": {
      "tau_input": 0.9,
      "tau_feat": 0.2,
      "logit_band_low": -0.05,
      "logit_band_high": 0.05
    }
  }'
```

You should get a JSON response like:

```json
{
  "ok": true,
  "trigger_acc": 0.96,
  "feat_dist": 0.04,
  "logit_stat": 0.01,
  "latency_ms": 123
}
```

(Exact numbers will vary but are deterministic for a given `(aid, evidence_hash)`.)

---

## Docker

A ready-to-use Dockerfile lives at `deploy/docker/Dockerfile.ml-service`:

```dockerfile
FROM python:3.11-slim

WORKDIR /app/ml_service

COPY ml_service/pyproject.toml .
COPY ml_service/src ./src

RUN pip install --no-cache-dir .

ENV ML_SERVICE_MODEL_ROOT=/app/ml_service/models
RUN mkdir -p /app/ml_service/models

EXPOSE 8080

CMD ["uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8080"]
```

From the workspace root:

```bash
docker build -f deploy/docker/Dockerfile.ml-service -t ml-service .
docker run --rm -p 8080:8080 ml-service
```

---

## Integration with the Rust Chain

The Rust `chain` crate uses `HttpMlVerifier` to call this service:

- It POSTs to `/verify` with `VerifyRequest` (same shape as above).
- It parses `VerifyResponse` and turns it into an `MlVerdict`.
- `MlValidity` uses `ok` to decide whether to accept a block.

By default, the Rust side assumes `base_url = "http://127.0.0.1:8080"`; in
Docker, you’ll eventually want to configure it to use the `ml-service`
container hostname instead (e.g. `http://ml-service:8080` on the `devnet`
network).

---

## Caveats

- Current verification is **not** a real watermarking scheme; it’s a stub
  that exercises the interface and basic performance characteristics.
- Error handling is deliberately simple: failed model loads result in
  `ok = false` with no distinction between “missing model” and “corrupted
  model”.
- No authentication or rate limiting yet – this is a research prototype
  endpoint, not a production service.
````
