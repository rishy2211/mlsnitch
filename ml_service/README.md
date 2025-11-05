# ml-service

Python ML authenticity verification service for the Rust `chain` prototype.

This service exposes:

- `POST /verify`

The Rust side (`HttpMlVerifier`) sends:

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
