# chain – ML-Aware Consensus Simulator

> Experimental Rust chain that **“bakes in” ML authenticity checks** directly into block validation.

This crate implements the consensus-layer side of a prototype blockchain where blocks are only valid if all newly-registered ML models pass a watermark-based authenticity check. It is designed to be:

- **Post-quantum friendly** – CRYSTALS-Dilithium / ML-DSA for signatures (via the Python side, not yet wired here).
- **Hash-safe by design** – all IDs are BLAKE3-256 newtypes (`Hash256`, `Aid`, `BlockHash`, etc.).
- **Modular** – consensus, validation, storage, ML client, metrics, and config are all separate subpackages.
- **Prototype-friendly** – small, testable components with clear traits so you can plug in mocks or real services.

This crate is the Rust side of the system; it expects a separate **Python + PyTorch** ML service that exposes a `/verify` endpoint for watermark checks.

---

## High-Level Architecture

At a very high level:

- **`types`** define the core domain objects:
  - `Block`, `Header`, `Transaction`, `TxRegisterModel`, `TxUseModel`, `TxTransfer`
  - `Aid` (model artefact ID), `EvidenceRef` (watermark evidence), `AccountId`, `Signature`
- **`consensus`** orchestrates:
  - `ConsensusEngine<S, V, F>` – generic over storage, validator, and fork-choice
  - `BlockStore` – abstraction for persistence
  - `BlockValidator` – trait for `V_base` and `V_cons`
  - `ForkChoice` – currently longest-chain-by-height
  - `Proposer` – builds blocks from a transaction pool
- **`validation`** contains:
  - `BaseValidity` – structural checks (size, tx count, duplicate `Aid`s in a block)
  - `MlValidity` – calls out to an ML verifier (`MlVerifier` trait) for authenticity checks
- **`storage`** provides:
  - `InMemoryBlockStore` – for tests and quick simulations
  - `RocksDbBlockStore` – persistent store with column families (`blocks`, `meta`)
- **`ml_client`** talks to the Python ML service:
  - `HttpMlVerifier` – blocking HTTP client implementing `MlVerifier`
- **`metrics`** defines:
  - `MetricsRegistry` + `ConsensusMetrics` – Prometheus metrics and a `/metrics` HTTP exporter
- **`config`** bundles node configuration:
  - `ChainConfig` – consensus, storage, ML client, metrics in one struct

The crate exposes default type aliases so a “typical” node can be wired up quickly:

```rust
pub type DefaultBlockValidator =
    CombinedValidator<BaseValidity, MlValidity<HttpMlVerifier>>;

pub type DefaultForkChoice = LongestChainForkChoice;
pub type DefaultBlockStore = RocksDbBlockStore;

pub type DefaultConsensusEngine =
    ConsensusEngine<DefaultBlockStore, DefaultBlockValidator, DefaultForkChoice>;
```
````

---

## Directory Layout

Inside the `chain/` crate:

```text
src/
  lib.rs           # crate root + re-exports + default type aliases
  main.rs          # demo node binary
  config.rs        # ChainConfig (consensus + storage + ML client + metrics)

  types/
    mod.rs         # Hash256, AccountId, Aid, EvidenceRef, WmProfile, ...
    block.rs       # Block, Header, BlockHash, canonical_bytes(), compute_hash()
    artefact.rs    # ArtefactMetadata (on-chain model registry entries)
    tx.rs          # TxRegisterModel, TxUseModel, TxTransfer, Transaction enum

  consensus/
    mod.rs         # re-exports
    config.rs      # ConsensusConfig (block time, max txs, max block size)
    error.rs       # ValidationError, ConsensusError
    store.rs       # BlockStore trait
    fork_choice.rs # ForkChoice, LongestChainForkChoice
    proposer.rs    # TxPool trait + Proposer (block construction)
    validator.rs   # BlockValidator, AcceptAllValidator, CombinedValidator
    engine.rs      # ConsensusEngine<S, V, F> + tests

  validation/
    mod.rs         # re-exports
    base.rs        # BaseValidity (block-local structural checks)
    ml.rs          # MlVerifier trait, MlValidity, MlConfig, MlError, MlVerdict

  storage/
    mod.rs         # re-exports
    mem.rs         # InMemoryBlockStore
    rocksdb.rs     # RocksDbBlockStore + RocksDbConfig + StorageError

  ml_client/
    mod.rs         # re-exports
    http.rs        # HttpMlVerifier (blocking reqwest client)

  metrics/
    mod.rs         # re-exports
    prometheus.rs  # MetricsRegistry, ConsensusMetrics, run_prometheus_http_server()
```

---

## Building & Running

### Prerequisites

- Rust (stable, recent; e.g. `rustup default stable`)
- A working C++ toolchain for RocksDB (e.g. `build-essential`, `clang`, etc. depending on OS)
- Optional but recommended:
  - Python ML service running at `http://127.0.0.1:8080` exposing `/verify`

### Build

From the crate root:

```bash
cargo build
```

### Run the demo node

The `main.rs` provided is a minimal node that:

- opens a RocksDB store at `data/chain-db` (by default),
- uses `BaseValidity + MlValidity<HttpMlVerifier>`,
- uses `LongestChainForkChoice`,
- exposes Prometheus metrics at `http://127.0.0.1:9898/metrics`,
- proposes empty blocks every `block_time_secs` seconds via an `EmptyTxPool`.

Run:

```bash
cargo run
```

You should see logs like:

```text
metrics exporter listening on http://127.0.0.1:9898/metrics
starting node with block_time_secs=5 (empty TxPool)
proposed block height=0 hash=...
proposed block height=1 hash=...
...
```

Hit the metrics endpoint:

```bash
curl http://127.0.0.1:9898/metrics
```

You’ll see metrics such as:

- `chain_consensus_block_validation_seconds`
- `chain_consensus_ml_auth_seconds`
- `chain_consensus_ml_cache_hit_ratio`
- `chain_consensus_blocks_rejected_ml`

(Names are prefixed with the `chain` namespace from the registry.)

---

## ML Service Contract

The Rust side expects an ML service with an HTTP endpoint similar to:

- **Request** (`POST /verify`):

```json
{
  "aid": "hex-encoded-aid",
  "scheme_id": "multi_factor_v1",
  "evidence_hash": "hex-encoded-evidence-hash",
  "wm_profile": {
    "tau_input": 0.9,
    "tau_feat": 0.1,
    "logit_band_low": 0.02,
    "logit_band_high": 0.05
  }
}
```

- **Response**:

```json
{
  "ok": true,
  "trigger_acc": 0.94,
  "feat_dist": 0.07,
  "logit_stat": 0.031,
  "latency_ms": 123
}
```

The client is implemented as `ml_client::HttpMlVerifier`, which turns these into `MlVerdict` values used by `MlValidity`.

You can plug in a different transport or protocol by implementing `validation::MlVerifier` yourself.

---

## Configuration

The top-level `ChainConfig` aggregates all node configuration:

```rust
pub struct ChainConfig {
    pub consensus: ConsensusConfig,
    pub storage: RocksDbConfig,
    pub ml_client: MlClientConfig,
    pub metrics: MetricsConfig,
}
```

Defaults are defined in `config.rs`:

- **ConsensusConfig**
  - `block_time_secs: 5`
  - `max_block_txs: 10_000`
  - `max_block_size_bytes: 1_000_000`
  - `allow_empty_blocks: true`

- **RocksDbConfig**
  - `path: "data/chain-db"`
  - `create_if_missing: true`

- **MlClientConfig**
  - `base_url: "http://127.0.0.1:8080"`
  - `timeout: 2s`

- **MetricsConfig**
  - `enabled: true`
  - `listen_addr: 127.0.0.1:9898`

In a real node binary, you’d typically:

- create `ChainConfig::default()`,
- override bits with CLI flags / env vars / TOML/YAML config,
- then wire everything up into a `DefaultConsensusEngine`.

---

## Extending the Chain

Some ideas for extending this crate:

- **Real transaction pool** – replace the `EmptyTxPool` demo with an in-memory mempool fed by RPC.
- **Real key management** – generate and store Dilithium keypairs; plug them into transaction signing.
- **Richer fork choice** – add weight-based fork choice (e.g. stake, cumulative work).
- **Enhanced ML validity** – extend `MlValidity` to:
  - consider multiple evidence sources (`V_wm`, `V_train`, `V_struct`),
  - integrate with a cache component (backed by RocksDB or in-memory),
  - add committee-style verification (subset of nodes produce signed certificates).

Because consensus/validation/storage/ML are all trait-based, you can experiment with these without changing the core types.

---

## Testing

Unit tests are included throughout the submodules:

- `types` – serde round-trips and hash determinism
- `tx` – bincode 2 encodings for all `Transaction` variants
- `block` – canonical hashing checks
- `consensus::engine` – fork-choice behaviour
- `validation::base` – block size / tx count / duplicate `Aid` checks
- `validation::ml` – `MlValidity` behaviour with a dummy verifier
- `storage::mem` and `storage::rocksdb` – store + tip round-trips
- `metrics::prometheus` – registry and encoding sanity checks
- `ml_client::http` – JSON parsing / hex encoding helpers

Run them with:

```bash
cargo test
```

---

## Caveats

This is a **research prototype**, not production-grade infrastructure. In particular:

- The consensus protocol is intentionally simplified (single-node friendly, longest-chain by height).
- There is no network stack here; propagation of transactions and blocks is out of scope.
- Security properties depend heavily on the external ML service and watermarking scheme.
