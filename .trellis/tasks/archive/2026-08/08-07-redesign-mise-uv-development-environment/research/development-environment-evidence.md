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

## Remote and release evidence

- PR #7 corrected Required CI run `31258884239` passed. Its predecessor
  `31258303784` failed closed in the new native Windows MSI fixture and was
  repaired without a compatibility fallback.
- Merge commit `bde1370bbaffd345c3d9875708615eaf96140591` passed exact-main
  Required CI run `31259389682`.
- Exact-main preflight `31259905022` passed Windows x64, Windows ARM64, Linux
  x64, Linux ARM64, and macOS Universal native build/package groups plus exact
  aggregation and attestation.
- Annotated `v0.3.0` tag object
  `e6706d4bdc33a184cf641204574df1fc2962ca4c` peels to that exact main source.
- Formal run `31260931509` published the stable, non-prerelease Release
  <https://github.com/NongHua123/fyagent/releases/tag/v0.3.0> with exactly ten
  installers and three evidence attachments. Independent re-download and all
  12 subject attestations passed.

These runs prove version/tag agreement, pinned Release toolchains, and native
release-package architecture. D117 observation also passed: each initiating
flow waited synchronously for the whole run to complete and then read final
results once; only the already-final failed PR run had failed-job logs
retrieved.

## Closeout native evidence and remaining boundary

The current host proves Linux x64 only, and no local non-host command was used.
A lock entry is not native execution evidence. The integrated
`windows-11-arm` Required job exercised the MSI query fixture, while the formal
Windows ARM64 Release job exercised Node/Rust compilation and MSI packaging;
neither ran uv-managed Python or a Trellis wrapper.

PR #8 <https://github.com/NongHua123/fyagent/pull/8> extends both Windows x64
and ARM64 Required legs with a Node plus locked uv/Python setup,
`uv sync --locked`, toolchain verification, and a Trellis task-list smoke before
the MSI fixture. Run `31264604075`, at head
`623b6924e3b8682321b26aa69c15dc6f0b9f6f09`, failed closed after x64 job
`93120609402` passed: ARM64 job `93120609411` failed because setup-uv's
version-only request selected `win-amd64`, and Required job `93121912798`
failed. Commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` switched to a full
uv request with managed Python. Run `31265504901` then passed x64 job
`93122857985`, ARM64 job `93122858012`, and Required job `93123992476`.

The original acceptance criterion “uv-managed Python and Trellis work on
Windows ARM64” is therefore complete. The formal Release is verified and GO,
the final design-package manifest is rebuilt and verified, and this child
remains active only until ordered archive. Archives, journal, final PR CI/merge,
exact-main CI, and branch cleanup remain pending closeout stages; D114 remains
N/A.
