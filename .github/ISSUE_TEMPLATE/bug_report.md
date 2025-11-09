---
name: "🐛 Bug report"
about: Report a bug in the ML-aware consensus prototype
title: "[BUG] "
labels: ["bug"]
assignees: []
---

## Summary

<!-- A clear and concise description of what the bug is. -->

## Environment

| Component      | Version / Info                          |
| -------------- | --------------------------------------- |
| chain (Rust)   | `cargo run -p chain --version` (if any) |
| api-gateway    | `cargo run -p api-gateway --version`    |
| ml_service     | `python -m src.main --version` (if any) |
| Rust toolchain | `rustc --version`                       |
| Python         | `python --version`                      |
| OS / Platform  | e.g. Ubuntu 22.04, macOS 15, Windows    |
| Docker         | `docker --version` (if relevant)        |

## Steps to Reproduce

1. <!-- e.g. Start ml_service on port 8080 -->
2. <!-- e.g. Run `cargo run -p api-gateway` -->
3. <!-- e.g. Call `/models/register` with payload X -->
4. <!-- Describe what happens -->

## Expected Behavior

<!-- A clear and concise description of what you expected to happen. -->

## Actual Behavior

<!-- What actually happened, including logs, errors, or screenshots if useful. -->

## Logs / Output

<details>
<summary>Relevant logs (click to expand)</summary>

```text
# Paste relevant logs here
```

```

</details>

## Configuration

<!-- Mention any non-default config you used, or attach snippets if relevant. -->

- `configs/devnet.toml`: <!-- yes/no, brief notes -->
- `configs/ml-service.toml`: <!-- yes/no, brief notes -->
- Custom env vars:
  - `ML_SERVICE_MODEL_ROOT`: <!-- value or "default" -->
  - `RUST_LOG`: <!-- value -->

## Additional Context

<!-- Add any other context, links, or notes about the problem here. -->
```
