# Redesign mise and uv development environment — PRD

## Goal

Provide a stable `mise run` repository API with standard version sources, uv-owned Python 3.14.7, strict checks, and cross-platform developer onboarding.

## Scope

- Node 24.19.0, pnpm 10.12.3, Rust 1.97.1 sources
- `uv = latest` with lock resolution
- pyproject/.python-version/uv.lock/.venv model
- full task namespaces and implementation scripts
- bootstrap/env/system/deps/check behavior
- Trellis and Codex hook wrappers
- task docs and lock structure tests

## Constraints

- Actions does not install mise
- auto-install is allowed
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

## Evidence Boundary

This task begins in `planning`. Nothing in this artifact claims the merge, configuration, tests, CI, or Release has already been completed. Pending platform/Git evidence must be attached during implementation.
