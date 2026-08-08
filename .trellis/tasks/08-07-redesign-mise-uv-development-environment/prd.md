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

- [x] supported platforms resolve exact toolchain
- [x] uv-managed Python and Trellis work on Windows ARM64
- [x] task DAG/metadata/docs checks pass
- [x] all current local docs use canonical tasks
- [x] Codex hooks are offline/no-sync and degrade visibly
- [x] Cargo workspace, both local lock packages, package metadata, version checks, and formal tag agree on `0.3.0`

## Evidence Boundary

Implementation commit `3d534710307d538e570c137231b1d80a64ac8ab7`
and the Linux x64/local-static evidence are recorded in
`research/development-environment-evidence.md`. The Child 6 active-document
migration is complete in `58101230e634e2280ef24937fad1942ed3d3d75f`. PR #7,
exact-main Required CI, the five-target native preflight, the annotated
`v0.3.0` tag, and the formal stable Release are verified. PR #8 run
`31264604075` at head `623b6924e3b8682321b26aa69c15dc6f0b9f6f09`
failed closed after x64 job `93120609402` passed: ARM64 job `93120609411`
failed because setup-uv's version-only request selected `win-amd64`, and
Required job `93121912798` failed. Commit
`4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` switched to a full uv request
with managed Python. Run `31265504901` then passed x64 job `93122857985`,
ARM64 job `93122858012`, and Required job `93123992476`. This is the native
execution evidence that closes the Windows ARM64 uv-managed Python and Trellis
criterion. The task remains `in_progress` only until its ordered archive;
manifest, archive, journal, final PR/main CI, merge, and cleanup stages are not
claimed here.
