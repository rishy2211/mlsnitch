## Summary

<!-- Briefly describe *what* this PR does. -->

- [ ] Bug fix
- [ ] Feature / enhancement
- [ ] Refactor / cleanup
- [ ] Docs / tooling

---

## Related Issues / Context

| Type   | Link / ID                               |
| ------ | --------------------------------------- |
| Issue  | <!-- e.g. #42 or "N/A" -->              |
| Design | <!-- e.g. doc link / thesis section --> |
| Other  | <!-- any extra context -->              |

---

## Changes

<!-- List the main changes in this PR. Bullets are good. -->

- `chain/`:
  - <!-- e.g. Added MlValidity timeout handling -->
- `api-gateway/`:
  - <!-- e.g. Added /models/{aid} status endpoint -->
- `ml_service/`:
  - <!-- e.g. Swapped stub watermark for real implementation -->
- `deploy/` / `configs/`:
  - <!-- e.g. Updated docker-compose ports -->

---

## Testing

<!-- Explain how you tested this change. Include commands & results. -->

### Rust

- [ ] `cargo fmt` (workspace)
- [ ] `cargo clippy --all-targets --all-features`
- [ ] `cargo test -p chain`
- [ ] `cargo test -p api-gateway`

### Python (`ml_service`)

- [ ] `pytest ml_service/tests`
- [ ] `black --check ml_service/src ml_service/tests`
- [ ] `ruff check ml_service/src ml_service/tests`

### Manual / Integration

| What                     | Command / Steps                                            | Result  |
| ------------------------ | ---------------------------------------------------------- | ------- |
| ML service health        | `curl http://localhost:8080/health`                        | ✅ / ❌ |
| API gateway health       | `curl http://localhost:8081/health`                        | ✅ / ❌ |
| Model registration flow  | `curl POST /models/register` + observe block producer logs | ✅ / ❌ |
| Metrics scrape           | `curl http://localhost:9898/metrics` / `9899/metrics`      | ✅ / ❌ |
| Docker devnet (optional) | `cd deploy && docker compose up --build`                   | ✅ / ❌ |

Add any logs or screenshots if helpful.

---

## Backwards Compatibility / Migration

- Does this change break any existing APIs (Rust or HTTP)?
  - [ ] No
  - [ ] Yes (details below)

If **yes**, describe:

- Breaking change:
- Affected clients / components:
- Migration steps:

---

## Checklist

- [ ] Code is formatted (`cargo fmt`, `black`)
- [ ] Lints are clean (`clippy`, `ruff`)
- [ ] Tests pass locally (Rust + Python)
- [ ] Docs / comments updated (where relevant)
- [ ] Configs / Docker updated (if behavior/ports changed)
- [ ] No secrets, keys, or sensitive data included

---

## Notes for Reviewers

<!-- Anything you want reviewers to pay special attention to?
     e.g. tricky logic, concurrency, error handling, perf tradeoffs. -->
