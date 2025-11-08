````markdown
# ML-Aware Consensus Prototype

This repo is a small, end-to-end prototype of a **blockchain that “bakes in” ML authenticity checks** at the consensus layer.

It consists of:

- a **Rust chain simulator** (`chain/`) with modular consensus, validation, storage, metrics, and an ML client,
- a **Rust API gateway** (`api-gateway/`) that exposes simple HTTP endpoints over the chain,
- a **Python FastAPI ML service** (`ml_service/`) that verifies watermarked models,
- **configs** for devnet and Prometheus, and
- **Docker** files to spin up a small local stack.

---

## High-Level Overview

| Component     | Tech                       | Role                                                           |
| ------------- | -------------------------- | -------------------------------------------------------------- |
| `chain`       | Rust                       | Core consensus engine + types + validation + storage + metrics |
| `api-gateway` | Rust + Axum                | HTTP API for clients (`/health`, `/models/register`)           |
| `ml_service`  | Python + FastAPI + PyTorch | ML authenticity service powering `V_auth`                      |
| `configs`     | TOML / YAML                | Devnet + API + ML + Prometheus configuration                   |
| `deploy`      | Docker                     | Dockerfiles + `docker-compose.yml` for running the full stack  |
| `docs`        | Markdown                   | Contributing / code of conduct                                 |
| `expts`       | (empty stub)               | Placeholder for experiments, scripts, notebooks, etc. (future) |

The overall flow:

1. A model owner trains a watermarked model and stores it in `ml_service`’s model directory.
2. A client calls `api-gateway` (`POST /models/register`), which queues a `TxRegisterModel`.
3. The consensus engine in `api-gateway` (or `chain`) proposes a block including that transaction.
4. During block validation, the Rust chain calls `ml_service`’s `/verify` endpoint.
5. If **all** model artefacts in the block pass `V_auth`, the block is accepted.

---

## Repo Structure

| Path           | Description                                                           |
| -------------- | --------------------------------------------------------------------- |
| `Cargo.toml`   | Rust workspace manifest (`chain` + `api-gateway`)                     |
| `LICENSE`      | Project license                                                       |
| `README.md`    | This file                                                             |
| `chain/`       | Rust chain library + binary (consensus, validation, storage, metrics) |
| `api-gateway/` | Rust HTTP API over the chain                                          |
| `ml_service/`  | Python FastAPI ML verification service                                |
| `configs/`     | TOML/YAML configs (devnet, Prometheus, ML service, API)               |
| `deploy/`      | Dockerfiles + `docker-compose.yml`                                    |
| `docs/`        | Meta docs (`CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`)                   |
| `expts/`       | Placeholder for experiments (scripts/notebooks/etc.)                  |

---

## Components in a Bit More Detail

### `chain/` – ML-Aware Chain Simulator (Rust)

Key modules:

| Module                  | Responsibility                                                                |
| ----------------------- | ----------------------------------------------------------------------------- |
| `types/`                | `Block`, `Header`, `Transaction`, `TxRegisterModel`, `Aid`, `EvidenceRef`…    |
| `consensus/`            | `ConsensusEngine`, `BlockStore`, `ForkChoice`, `Proposer`, validators         |
| `validation/base.rs`    | Cheap block-local checks (`V_base`: size, tx count, duplicate `Aid`, …)       |
| `validation/ml.rs`      | ML authenticity checks (`V_auth` via `MlVerifier`) and per-block artefact cap |
| `storage/mem.rs`        | In-memory `BlockStore` for tests/dev                                          |
| `storage/rocksdb.rs`    | RocksDB-backed `BlockStore` for persistent nodes                              |
| `ml_client/http.rs`     | HTTP client (`HttpMlVerifier`) for `ml_service`’s `/verify` endpoint          |
| `metrics/prometheus.rs` | `MetricsRegistry` + `/metrics` exporter                                       |
| `config.rs`             | `ChainConfig` (consensus + storage + ML client + metrics)                     |
| `main.rs`               | Minimal demo node (RocksDB + metrics + block loop)                            |

### `api-gateway/` – HTTP Frontend (Rust)

| File                   | Responsibility                                                             |
| ---------------------- | -------------------------------------------------------------------------- |
| `src/main.rs`          | Builds consensus engine, metrics, tx pool, routes, and block producer loop |
| `src/config.rs`        | `ApiConfig` (HTTP listen address)                                          |
| `src/state.rs`         | `AppState` (`engine`, `tx_pool`, `proposer_id`, `metrics`)                 |
| `src/routes/health.rs` | `GET /health`                                                              |
| `src/routes/models.rs` | `POST /models/register` → queue `TxRegisterModel`                          |
| `README.md`            | Component-specific docs                                                    |

### `ml_service/` – ML Authenticity Service (Python/FastAPI)

| File / Package                     | Responsibility                                                                    |
| ---------------------------------- | --------------------------------------------------------------------------------- |
| `src/main.py`                      | FastAPI app (`/health`, `/verify`)                                                |
| `src/schemas.py`                   | Pydantic models: `WmProfile`, `VerifyRequest`, `VerifyResponse`, `HealthResponse` |
| `src/config.py`                    | `MODEL_ROOT` (`ML_SERVICE_MODEL_ROOT` env var)                                    |
| `src/registry/filesystem_store.py` | Maps `aid_hex` → `<MODEL_ROOT>/<aid_hex>.pt`                                      |
| `src/watermark/verify.py`          | Stubbed multi-factor watermark verifier (deterministic stats + thresholds)        |
| `src/models/resnet.py`             | Example `SmallResNet` architecture (for future training/integration)              |
| `tests/`                           | pytest suite for schemas, watermark, and FastAPI app                              |
| `pyproject.toml`                   | Package metadata + dependencies                                                   |
| `environment.yml`                  | Conda environment for local dev                                                   |

---

## How the Stack Fits Together

| Service       | Port (host)    | Exposed endpoints                                                        | Notes                                  |
| ------------- | -------------- | ------------------------------------------------------------------------ | -------------------------------------- |
| `ml_service`  | `8080`         | `GET /health`, `POST /verify`                                            | Python FastAPI, used by Rust ML client |
| `chain`       | `9898`         | `GET /metrics`                                                           | Rust node metrics (Prometheus)         |
| `api-gateway` | `8081`, `9899` | `GET /health`, `POST /models/register`, `/metrics` (9899→container 9898) | Rust API + embedded consensus          |
| `prometheus`  | `9090`         | Prometheus web UI                                                        | Scrapes `chain` + `api-gateway`        |

> In plain Rust-only dev (no Docker), the default `ChainConfig` assumes the ML service is at `http://127.0.0.1:8080`.

---

## Quickstart (No Docker)

### 0. Prerequisites

- Rust (stable)
- Python 3.10+ with `venv` (or conda)
- Locally running `ml_service` (see below)

### 1. Start the ML Service

```bash
cd ml_service

# Create & activate env (conda or venv)
conda env create -f environment.yml   # or python -m venv .venv && source .venv/bin/activate
conda activate ml-service

# Install the package
pip install -e .

# Make a dummy model file
mkdir -p models
touch models/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.pt

# Run FastAPI app
uvicorn src.main:app --host 0.0.0.0 --port 8080
```
````

### 2. Run the Rust chain node

```bash
# From repo root
cargo run -p chain
```

This:

- opens a RocksDB store under `data/chain-db`,
- exposes metrics on `http://127.0.0.1:9898/metrics`,
- proposes (currently empty) blocks every few seconds.

### 3. Run the API gateway

In another terminal:

```bash
# From repo root
cargo run -p api-gateway
```

You’ll see logs like:

```text
metrics exporter listening on http://0.0.0.0:9898/metrics
API gateway listening on http://127.0.0.1:8081
block producer running with interval 5s
```

### 4. Test the endpoints

Health:

```bash
curl http://127.0.0.1:8081/health
# {"status":"ok"}
```

Register a model:

```bash
curl -X POST http://127.0.0.1:8081/models/register \
  -H "Content-Type: application/json" \
  -d '{
    "owner_account_hex": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aid_hex": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "scheme_id": "multi_factor_v1",
    "evidence_hash_hex": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    "wm_profile": {
      "tau_input": 0.9,
      "tau_feat": 0.2,
      "logit_band_low": -0.05,
      "logit_band_high": 0.05
    }
  }'
```

You should get:

```json
{
  "status": "queued",
  "aid": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
}
```
