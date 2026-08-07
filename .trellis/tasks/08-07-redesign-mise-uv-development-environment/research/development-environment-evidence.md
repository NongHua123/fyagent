# Development environment modernization evidence

## Result

Commit `3d534710307d538e570c137231b1d80a64ac8ab7` establishes the
FyAgent 0.3.0 local development contract:

- Node.js `24.19.0` from `.node-version`
- pnpm `10.12.3` from `package.json#packageManager`
- Rust `1.97.1` with the minimal profile, rustfmt, and Clippy from
  `rust-toolchain.toml`
- Python `3.14.7` from `.python-version`, owned by uv rather than mise or the
  system interpreter
- `uv = "latest"` as the only direct tool in `mise.toml`, resolved to uv
  `0.12.2` by the reviewed `mise.lock`
- product version `0.3.0` in the Cargo workspace source and both local
  Cargo.lock packages

The implementation adds a non-package uv project, 80 canonical `mise run`
tasks, strict environment/system checks, generated task documentation,
locked/no-sync/offline Codex hooks, preview-by-default maintenance and version
commands, and long-term executable contracts.

## Lock evidence

`mise.lock` was generated from an empty prior-lock state for all six supported
platform keys and then regenerated a second time without changing bytes. Its
SHA-256 is:

```text
5f0d9df527ec1fdaf5532726ba30d330c74872786ad0380783064a36ceeefd9d
```

Node, pnpm, and uv have generated URLs and checksums for Linux x64/ARM64,
macOS x64/ARM64, and Windows x64/ARM64. The Windows ARM64 entries select
`pnpm-win-arm64.exe` and `uv-aarch64-pc-windows-msvc.zip`. The `core:rust`
backend publishes no lockable platform assets, so the lock records the exact
Rust version/options and native runners must separately prove the rustup
toolchain; no checksum was fabricated.

The final `mise run bootstrap` preserved all tracked locks byte-for-byte:

```text
mise.lock             5f0d9df527ec1fdaf5532726ba30d330c74872786ad0380783064a36ceeefd9d
uv.lock               fec8c5fbb89134ebc74f648217785151c22704459bb1363670630d76582b1e23
pnpm-lock.yaml        fb566ae1ee6a3d15d29f8da33543dbeb733fa13346074f7fbc69e8527ce03f82
src-tauri/Cargo.lock  5b270ae87d8806c112028a1293b8050c94378735cbef308c8837b01affd535b7
```

## Independent check

The formal Trellis check fixed five failure-boundary groups before final
verification:

1. `system:check` now converts missing or unlaunchable prerequisite commands
   into failed JSON checks with installation hints instead of aborting on
   ENOENT.
2. Hook input containment rejects Windows cross-drive paths and symlink escapes.
3. A symlink or junction `.venv` fails closed rather than allowing writes
   outside the side-effect snapshot.
4. Codex strict SubagentStart failures remain closed while the Python hook's
   established non-Codex generic fallback remains available.
5. `version:check` rejects duplicate critical TOML sections and duplicate
   local Cargo.lock package fields instead of accepting structurally ambiguous
   version metadata.

The earlier contract audit also closed arbitrary Python-command effect
misclassification, write-capable Vitest option forwarding, Rust foreign-target
injection, hook TOML wrong-table acceptance, and direct non-atomic version-file
replacement.

## Verification

The final frozen worktree passed:

- `mise run bootstrap`, including a before/after SHA-256 comparison proving no
  tracked lock mutation
- `mise run check` with environment, frontend, backend, and contract gates
- `pnpm test:unit -- --reporter=dot` — 134 test files and 919 tests passed
- `cargo fmt --all --check`, locked workspace/all-targets Cargo check, strict
  Clippy with `-D warnings`, and locked workspace tests through the canonical
  Rust task DAG
- `pnpm typecheck` and the repository frontend format check
- explicit Prettier checks for the new task scripts, tests, generated task
  reference, and affected long-term specs
- `mise run tasks:validate` — all 80 tasks, effects, parameter contracts,
  read-only `check` closure, docs boundary, and lock structure passed
- `mise run tasks:docs:check` — generated task documentation matched byte for
  byte
- `mise run check:contracts` — six files and 47 tests passed after the final
  generator-format correction
- `node --test tests/version.test.mjs` — 19/19 passed
- `mise run version:check --tag v0.3.0`
- `mise run codex:hooks:check`
- `mise run env:check --json` — all nine Linux x64 checks passed
- `python3 .trellis/scripts/task.py validate
.trellis/tasks/08-07-redesign-mise-uv-development-environment`
- `git diff --cached --check` and the unstaged diff check

The managed command environment reported:

```text
product  0.3.0
Node.js  24.19.0
pnpm     10.12.3
Rust     1.97.1
Python   3.14.7
uv       0.12.2
```

## Evidence boundary

The current host proves Linux x64 only. A lock entry is not native execution
evidence: Windows x64/ARM64, Linux ARM64, and macOS must still run locked setup,
environment ownership, hooks, and canonical tasks in GitHub Actions. Active
README/CONTRIBUTING/spec examples that remain on the explicit handoff allowlist
belong to `08-07-migrate-docs-and-trellis-specs`. The actual `v0.3.0` tag and
formal Release have not occurred. Those unchecked acceptance items keep this
child open and prevent its archive from overstating completion.
