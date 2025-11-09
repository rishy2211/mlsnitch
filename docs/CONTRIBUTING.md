# Contributing

Thanks for your interest in contributing! 🎉  
This project combines a Rust-based chain and an ML verification service, so contributions across systems, infra, and ML are all welcome.

This document outlines how to get set up, how we work, and what we expect from contributions.

---

## Code of Conduct

By participating, you agree to follow our [Code of Conduct](./CODE_OF_CONDUCT.md).

If you experience or witness unacceptable behaviour, please reach out via the contact details in the Code of Conduct.

---

## Ways to Contribute

- 🐛 **Bug reports** – incorrect behaviour, crashes, surprising UX, flaky tests
- 💡 **Feature suggestions** – improvements to the chain, API gateway, or ML service
- 🧪 **Tests** – more coverage and better regression protection
- 🧰 **Tooling & docs** – CI, dev tooling, docs, examples, quickstart guides

Before starting a large feature, consider opening a discussion or issue so we can align on the design.

---

## Project Layout (High Level)

- `chain/` – Rust crate for the chain, consensus engine, storage, validation, metrics, ML client, etc.
- `api-gateway/` – Rust HTTP gateway for clients (health, model registration/use, etc.)
- `ml_service/` – Python ML service (artefact registry, watermark verifier, models)
- `config/` – TOML configs for local/dev services
- `deploy/` – Dockerfiles and deployment wiring
- `docs/` – Additional documentation (Code of Conduct, contributing notes, etc.)

See the main `README.md` for a more detailed repo structure and quickstart.

---

## Getting Started

### 1. Fork & Clone

```bash
git clone https://github.com/<your-username>/<repo-name>.git
cd <repo-name>
```

````

### 2. Create a Branch

Use a descriptive branch name:

```bash
git checkout -b feat/better-metrics-endpoint
# or
git checkout -b fix/rocksdb-tip-panic
```

---

## Development Environment

### Rust Tooling (chain & api-gateway)

Requirements (roughly):

- Rust (stable, via [rustup](https://rustup.rs/))
- `cargo` for builds and tests

Useful commands:

```bash
# From repo root
cargo build --workspace
cargo test --workspace

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features
```

### Python Tooling (ml_service)

Typical flow (adapt to your stack):

```bash
cd ml_service

# Option 1: virtualenv
python -m venv .venv
source .venv/bin/activate

# Option 2: Conda
# conda env create -f environment.yml
# conda activate <env-name>

# Install the service in dev mode
pip install -e ".[dev]"

# Run tests
pytest
```

Check `ml_service/README.md` and `pyproject.toml` for exact extras and commands.

---

## Running the Stack with Docker

If you prefer containers:

```bash
# From repo root
docker compose up --build
```

This should spin up:

- The chain node
- The API gateway
- The ML service

See the main README for port mappings and configuration options.

---

## Commit Messages

We roughly follow a conventional style to keep history readable:

**Format:**

```text
type(scope): short description

Longer body explaining the change, rationale, and any migration notes.
```

**Common types:**

- `feat` – new feature
- `fix` – bug fix
- `docs` – documentation only
- `test` – tests only
- `chore` – tooling, refactors, CI, etc.
- `ci` – GitHub Actions / CI config
- `build` – build system or dependency changes
- `refactor` – internal code change with no behavioural difference

Examples:

- `feat(chain-consensus): add proposer backoff for empty slots`
- `chore(ml-service): tighten dependency pins`
- `ci: add Rust and Python code-check workflows`

---

## Before You Push

Please run (from the repo root, or as appropriate):

- For Rust:

  ```bash
  cargo fmt --all
  cargo clippy --all-targets --all-features
  cargo test --workspace
  ```

- For Python (in `ml_service/`):

  ```bash
  pytest
  # plus any format/lint tools you have in the project, e.g.:
  # ruff check .
  # black .
  ```

If you use [`act`](https://github.com/nektos/act) locally, you can also run the GitHub Actions workflows before pushing to get the same checks CI runs.

---

## Pull Request Guidelines

When you open a PR:

1. **One logical change per PR** where possible (small, reviewable units).
2. Ensure:
   - CI passes
   - New tests are added for new behaviour
   - Docs / comments are updated where behaviour changed

3. Include:
   - A clear description of _what_ changed and _why_
   - Notes about breaking changes or migrations, if any

4. Be open to feedback and iteration 🙂.

---

## Reporting Bugs

If you’re not sure whether something is a bug or expected behaviour:

- Search existing issues first (to avoid duplicates).
- When opening a new issue, include:
  - Steps to reproduce
  - Expected vs actual behaviour
  - Logs or error messages (redacted if needed)
  - Your environment (OS, Rust/Python versions, Docker vs bare metal, etc.)

---

## Security Issues

For **security-related** issues or potential vulnerabilities, **do not open a public issue**.
Instead, please follow the process in [SECURITY.md](./SECURITY.md).

---

## Thanks 💜

Your time and contributions are hugely appreciated.
Whether it’s fixing a typo, improving tests, or designing new consensus/ML features, thank you for helping improve this project.
````
