# Redesign mise and uv development environment — PRD

## Goal

Provide a stable `mise run` repository API with standard version sources, uv-owned Python 3.14.7, strict checks, and cross-platform developer onboarding.

## Scope

- exact standard sources: Node 24.19.0, pnpm 10.12.3, Rust 1.97.1, and Python 3.14.7
- `mise.toml` declares no duplicate Node/pnpm/Rust/Python versions and owns only `uv=latest`, task includes, and necessary settings
- `mise.lock` pins approved uv assets/checksums across supported platforms; uv exclusively owns `.python-version`, the non-package `pyproject.toml`, `uv.lock`, and `.venv`
- `uv = latest` with lock resolution
- pyproject/.python-version/uv.lock/.venv model
- full task namespaces and implementation scripts
- bootstrap/env/system/deps/check behavior
- Trellis and Codex hook wrappers
- product version tooling with dry-run by default and explicit `--apply`, updating the Cargo workspace and local lock packages atomically to `0.3.0`
- task docs and lock structure tests

## Constraints

- Actions does not install mise
- tool installation through explicit bootstrap is allowed, but bootstrap never trusts config, installs system packages, changes remotes, refreshes locks, or publishes
- env/system checks are strict/read-only
- no system Python fallback
- no global `.venv` injection
- no task grants trust or modifies remotes

## Acceptance Criteria

- [ ] supported platforms resolve exact toolchain
- [ ] uv-managed Python and Trellis work on Windows ARM64
- [ ] task DAG/metadata/docs checks pass
- [ ] all current local docs use canonical tasks
- [ ] Codex hooks are offline/no-sync and degrade visibly
- [ ] Cargo workspace, both local lock packages, package metadata, version checks, and eventual tag agree on `0.3.0`

## Evidence Boundary

Implementation was authorized on 2026-08-08. Authorization is not completion evidence: exact versions, locks, tasks, hooks, the atomic `0.3.0` update, and available platform results remain pending until verified.
