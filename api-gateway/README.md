# api-gateway – HTTP Frontend for the ML-Aware Chain

This crate exposes a small **HTTP API** on top of the Rust `chain` crate:

- `GET /health` – liveness check
- `POST /models/register` – queue a `TxRegisterModel` into the consensus
  engine

Behind the scenes it embeds:

- a `DefaultConsensusEngine` (RocksDB-backed),
- `BaseValidity` + `MlValidity<HttpMlVerifier>` for block validation,
- a simple FIFO transaction pool,
- a background block producer loop, and
- a Prometheus metrics exporter (via the `chain` crate).

The goal is to give clients a simple way to register ML models on-chain
without dealing with consensus internals.

---

## Architecture

The `api-gateway` binary links directly against the `chain` crate:

- **Consensus** (`chain::ConsensusEngine`):
  - storage: `RocksDbBlockStore` at `data/chain-db` (by default)
  - validator: `CombinedValidator<BaseValidity, MlValidity<HttpMlVerifier>>`
  - fork choice: `LongestChainForkChoice` (longest chain by height)
- **ML verification** (`chain::ml_client::HttpMlVerifier`):
  - base URL: `ChainConfig::default().ml_client.base_url`
    (`http://127.0.0.1:8080` by default)
- **Metrics** (`chain::metrics`):
  - `MetricsRegistry` shared with the consensus engine
  - HTTP exporter on `ChainConfig::default().metrics.listen_addr`
    (`0.0.0.0:9898` by default)
- **Tx pool**:
  - `QueuedTxPool` — FIFO queue of `Transaction`s
- **HTTP**:
  - `axum` router with `/health` and `/models/register`

Block production is handled by a background task that calls:

```rust
engine.propose_block(proposer_id, &mut tx_pool, timestamp)
```

every `block_time_secs` seconds (default 5s).

---

## API

### `GET /health`

Simple liveness check (does not touch consensus).

**Response:**

```json
{
  "status": "ok"
}
```

---

### `POST /models/register`

Queue a `TxRegisterModel` transaction into the local transaction pool; the
block producer will eventually include it in a block (subject to validity
checks and capacity).

**Request body**:

```json
{
  "owner_account_hex": "hex-encoded-account-id",
  "aid_hex": "hex-encoded-aid",
  "scheme_id": "multi_factor_v1",
  "evidence_hash_hex": "hex-encoded-evidence-hash",
  "wm_profile": {
    "tau_input": 0.9,
    "tau_feat": 0.1,
    "logit_band_low": -0.05,
    "logit_band_high": 0.05
  }
}
```

Fields:

- `owner_account_hex` – 64 hex chars (32-byte `AccountId`).
  - In the chain, `AccountId` is `Hash256` (BLAKE3-256 of a Dilithium
    public key). For testing you can pick any valid 64-char hex string.

- `aid_hex` – 64 hex chars (32-byte `Aid`).
  - In the chain, `Aid` is `Hash256` of the model bytes. For demos you
    can pick any valid value, as long as it matches the model name used
    in the ML service.

- `scheme_id` – watermark scheme identifier, e.g. `"multi_factor_v1"`.
- `evidence_hash_hex` – 64 hex chars (32-byte `EvidenceHash`).
- `wm_profile` – tuning parameters used by the ML watermark detector.

**Response** (202 Accepted):

```json
{
  "status": "queued",
  "aid": "hex-encoded-aid"
}
```

This only guarantees the transaction has been queued locally. It does _not_
wait for the transaction to be included in a block or for the ML check to
pass; that’s handled asynchronously by the consensus engine and ML service.

---

## Code Layout

```text
src/
  main.rs      # binary entrypoint: builds engine, tx pool, metrics, router
  config.rs    # ApiConfig (listen_addr for HTTP server)
  state.rs     # AppState (engine + tx pool + proposer_id + metrics)

  routes/
    mod.rs
    health.rs  # GET /health
    models.rs  # POST /models/register
```

Key pieces:

- `AppState` (in `state.rs`):
  - `engine: Mutex<DefaultConsensusEngine>`
  - `tx_pool: Mutex<QueuedTxPool>`
  - `proposer_id: AccountId`
  - `metrics: Arc<MetricsRegistry>`

- `QueuedTxPool` implements `chain::TxPool` and stores a `VecDeque<Transaction>`.

- `run_block_producer` (in `main.rs`) loops:
  1. Locks `engine` and `tx_pool`.
  2. Calls `engine.propose_block(..., &mut tx_pool, timestamp)`.
  3. Records `block_validation_seconds` in the metrics registry.
  4. Sleeps `block_time_secs`.

---

## Running Locally (without Docker)

Prerequisites:

- Rust (stable)
- A running ML service (e.g. the `ml_service` FastAPI app) listening on
  `http://127.0.0.1:8080`

From the workspace root:

```bash
# Run the ml_service in another terminal:
cd ml_service
uvicorn src.main:app --host 0.0.0.0 --port 8080

# Back in the workspace root:
cargo run -p api-gateway
```

You should see logs like:

```text
metrics exporter listening on http://0.0.0.0:9898/metrics
API gateway listening on http://127.0.0.1:8081
block producer running with interval 5s
```

### Health check

```bash
curl http://127.0.0.1:8081/health
# {"status":"ok"}
```

### Registering a model

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

Shortly after, the block producer will propose a block that includes this
`TxRegisterModel` (assuming ML checks pass).

---

## Metrics

The gateway uses the same `MetricsRegistry` as the consensus engine, and
exposes `/metrics` via the `chain` crate’s HTTP exporter.

- Inside the container: `0.0.0.0:9898/metrics`
- In the provided `docker-compose.yml`, this is mapped to host `9899`:

  ```yaml
  api-gateway:
    ports:
      - "8081:8081"
      - "9899:9898"
  ```

So you can hit:

```bash
curl http://127.0.0.1:9899/metrics
```

You’ll see metrics like:

- `chain_consensus_block_validation_seconds`
- `chain_consensus_ml_auth_seconds`
- `chain_consensus_ml_cache_hit_ratio`
- `chain_consensus_blocks_rejected_ml`

---

## Docker

The gateway has its own Dockerfile at `deploy/docker/Dockerfile.api-gateway`:

```dockerfile
FROM rust:1.81 as builder
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY chain/Cargo.toml chain/Cargo.toml
COPY api-gateway/Cargo.toml api-gateway/Cargo.toml
COPY . .
RUN cargo build -p api-gateway --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/src/app/target/release/api-gateway /usr/local/bin/api-gateway
ENV RUST_LOG=api_gateway=info,chain=info
EXPOSE 8081 9898
CMD ["api-gateway"]
```

In `deploy/docker-compose.yml` the service is defined as:

```yaml
api-gateway:
  build:
    context: ..
    dockerfile: deploy/docker/Dockerfile.api-gateway
  container_name: api-gateway
  working_dir: /app
  ports:
    - "8081:8081"
    - "9899:9898"
  environment:
    - RUST_LOG=api_gateway=info,chain=info
  depends_on:
    - chain
    - ml-service
  networks:
    - devnet
```

From `deploy/`:

```bash
docker compose up --build
```

You’ll get:

- API: `http://localhost:8081`
- API metrics: `http://localhost:9899/metrics`
- Chain metrics: `http://localhost:9898/metrics`
- ML service: `http://localhost:8080`
- Prometheus UI: `http://localhost:9090`

---

## Caveats

- Signature verification is currently stubbed: `TxRegisterModel.signature`
  is set to an empty `Signature(Vec<u8>)`. In a full implementation this
  would be a Dilithium signature over a canonical transaction encoding.
- There is no authentication or rate limiting; this is a research
  prototype, not a production API.
- All configuration currently uses Rust defaults and env vars; the TOML
  configs (`configs/api.toml`, `configs/devnet.toml`) are provided for
  future wiring, but not yet parsed by the code.
