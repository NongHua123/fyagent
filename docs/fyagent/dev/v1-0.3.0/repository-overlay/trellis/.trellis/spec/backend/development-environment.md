# Development Environment Contract

## Purpose

This contract owns the reproducible local development toolchain. It does not make mise a packaged FyAgent runtime dependency and does not govern CC Switch-compatible optional CLI discovery in the product.

## Version sources of truth

| Tool | Required target | Authoritative declaration |
|---|---:|---|
| Node.js | `24.19.0` | `.node-version` |
| pnpm | `10.12.3` | `package.json#packageManager` |
| Rust | `1.97.1` | `rust-toolchain.toml` |
| uv | `latest` selector; resolved version committed | `mise.toml` + `mise.lock` |
| Python | `3.14.7` | `.python-version`, consumed only by uv |
| Python compatibility | `>=3.14,<3.15` | `pyproject.toml#requires-python` |

`mise.toml` MUST NOT duplicate Node, pnpm, Rust, or Python versions. It enables the standard Node/package/Rust version files and declares uv. `mise.lock` is a generated resolution artifact, not a competing source of truth.

## Management chain

```text
mise
├─ selects Node, pnpm, Rust, and uv
├─ exposes canonical `mise run` tasks
└─ permits normal automatic installation

uv
├─ exclusively installs/selects Python
├─ creates project-root .venv
├─ consumes pyproject.toml and .python-version
└─ owns uv.lock and Python package synchronization
```

The project MUST NOT fall back to system Python. Python commands in Trellis and hooks run through uv. The `.venv` is not globally injected into Node/Rust/Git tasks and never needs manual activation.

## Trust and installation

- A user reviews configuration and runs `mise trust` explicitly.
- No repository task, script, hook, or product code may execute `mise trust --yes` or alter trust state.
- Normal mise automatic installation is allowed.
- `bootstrap` does not install privileged host packages or modify Git remotes.
- Local overrides (`mise.local.toml`, `mise.local.lock`, `mise.*.local.toml`, `mise.*.local.lock`) and `.venv/` are ignored by Git.

## Required tasks

```text
mise run bootstrap
mise run env:check
mise run system:check
mise run deps:install
mise run check
```

`env:check` is strict, read-only, non-repairing, and returns nonzero on mismatch. It validates declarations, actual versions, active tool origins, lock structure, uv-managed Python, `.venv`, task metadata, and absence of retired cross targets/`llvm-tools`. It emits human-readable output and machine-readable JSON from the same rule engine.

`system:check` is strict and read-only. It checks only current-host Tauri prerequisites and prints official remediation guidance; it never performs privileged installation.

## Lockfiles

- `pnpm-lock.yaml`, `Cargo.lock`, `mise.lock`, and `uv.lock` are committed.
- Ordinary checks use frozen/locked modes and do not rewrite a lockfile.
- `mise.lock` is generated, structurally parsed, and checked per tool/platform/options; substring-only tests are insufficient.
- uv daily execution uses `uv sync --locked` or `uv run --locked`; lock refresh is explicit.
- `uv = "latest"` is resolved through `mise.lock`; upgrades occur only through a reviewed lock-bump PR.

## Platform contract

Windows ARM64 is a supported local development platform for mise, uv-managed Python, Trellis, and contract checks. Native application builds are current-host only. No base environment installs Apple/Windows cross targets or `llvm-tools` for all contributors.

## Tests Required

- strict declaration and runtime version test on each CI platform;
- structural `mise.lock` and `uv lock --check` tests;
- Windows ARM64 uv/Python/Trellis smoke test where supported;
- negative test for duplicate tool versions and retired cross targets;
- negative test proving `env:check` does not accept an unrelated same-version executable;
- documentation/task consistency test.
