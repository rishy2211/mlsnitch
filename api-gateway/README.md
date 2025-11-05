chain/                     # Rust chain simulator & validator node
├─ Cargo.toml
└─ src/
   ├─ main.rs              # start a validator node / local devnet
   ├─ lib.rs
   ├─ types/
   │  ├─ block.rs          # Block, Header, ML(B) helper
   │  ├─ tx.rs             # TxRegisterModel, TxUseModel, account txs
   │  └─ artefact.rs       # Aid, EvidenceProfile, on-chain metadata
   ├─ consensus/
   │  ├─ mod.rs
   │  ├─ proposer.rs       # simple PoS-style proposer loop
   │  └─ fork_choice.rs    # longest-chain / heaviest-chain
   ├─ validation/
   │  ├─ mod.rs
   │  ├─ base.rs           # V_base(B)
   │  └─ ml.rs             # MlValidity: calls V_auth via ml_client
   ├─ storage/
   │  ├─ mod.rs
   │  ├─ rocksdb.rs        # CFs: blocks, artefacts, ml_cache
   │  └─ state.rs          # account state, artefact index
   ├─ ml_client/
   │  ├─ mod.rs
   │  └─ http_client.rs    # async HTTP client for /verify
   ├─ metrics/
   │  ├─ mod.rs
   │  └─ prometheus.rs     # /metrics endpoint, histograms, gauges
   └─ config.rs            # node config (ports, db path, timeouts
