```markdown
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
```
