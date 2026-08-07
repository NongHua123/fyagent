# Development Environment Contract

## 1. Scope / Trigger

Read this contract before changing repository tool versions, `mise.toml`, any
lockfile, Python/Trellis execution, local onboarding commands, WSL behavior, or
the canonical task API. It applies to native development on Linux, macOS,
Windows, and WSL. GitHub Actions deliberately installs tools with native setup
actions instead of installing mise.

## 2. Authoritative Version Sources

Each standard ecosystem file is the only human-maintained version source for
its tool:

```text
.node-version                    Node.js 24.19.0
package.json#packageManager      pnpm@10.12.3
rust-toolchain.toml              Rust 1.97.1, minimal + rustfmt + clippy
.python-version                  Python 3.14.7 (consumed only by uv)
mise.toml#[tools]                uv = "latest"
mise.lock                        approved uv resolution and tool artifacts
```

`mise.toml` must not repeat Node, pnpm, Rust, or Python versions. It enables the
Node, pnpm, and Rust idiomatic files, declares only `uv = "latest"`, includes
the domain task TOMLs, and disables automatic installation when ordinary tasks
start. The repository requires mise `>= 2026.8.0`; repository scripts never
download or privately install mise.

The audited repository aliases are:

```toml
[tool_alias]
pnpm = "github:pnpm/pnpm"
uv = "github:astral-sh/uv"
```

They are required because the default aqua resolution observed with mise
2026.8.1 selected x64 Windows assets under a `windows-arm64` key even though
both upstream releases publish native ARM64 assets. A lock regenerated from an
empty file through the aliases selects `pnpm-win-arm64.exe` and
`uv-aarch64-pc-windows-msvc.zip`; `lockfile-check.mjs` rejects a platform key
whose URL names another architecture.

`mise.lock` targets `linux-x64`, `linux-arm64`, `macos-x64`, `macos-arm64`,
`windows-x64`, and `windows-arm64`. Node, pnpm, and uv have a generated HTTPS
URL and SHA-256 checksum for every platform. The `core:rust` backend currently
emits no platform artifact records: mise reports those six entries as skipped,
so the lock stores exact Rust version/options and native jobs must additionally
prove the selected rustup toolchain. Do not fabricate Rust checksums or present
the platform list alone as artifact evidence.

## 3. uv-owned Python Project

The repository is not a Python package. `pyproject.toml` defines an empty
development environment with `requires-python = ">=3.14,<3.15"`, an empty
`dev` dependency group, and:

```toml
[tool.uv]
package = false
python-preference = "only-managed"
python-downloads = "automatic"
```

uv exclusively owns Python selection, downloads, `.venv`, project dependencies,
and `uv.lock`. There is no system-Python fallback and mise does not inject
`.venv` into every task. Repeatable Python dependencies enter
`pyproject.toml`/`uv.lock`; one-off dependencies use `python:with` or
`python:tool`.

Ordinary Python and Trellis tasks use `uv run --locked` and may prepare the
locked environment. Codex hooks use
`uv run --locked --no-sync --offline`: an unprepared environment returns an
explicit, non-blocking fallback, while malformed hook code or protocol output
fails closed.

## 4. Setup and Execution Boundaries

After independently reviewing the repository config, a developer may trust it
once outside any repository task. The standard flow is:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

No task runs `mise trust` or `mise untrust`. `bootstrap` is the only high-level
environment preparation task. It may run locked mise installation, frozen pnpm
installation, `uv sync --locked`, strict environment checks, and task
validation. It must not install system packages, change trust, change Git
remotes, refresh locks, build, tag, sign, upload, or publish.

`env:check` is strict and read-only. It verifies the standard sources, actual
versions, executable ownership, WSL path isolation, generated lock structure,
uv-managed Python, `.venv`, offline locked Python execution, Rust components
and sysroot, mise task metadata, and Codex hook task presence. `--json` emits
one machine-readable report and any failed check exits nonzero.

`system:check` is strict and read-only. It probes current-host Tauri
prerequisites and prints official package/tool hints; it never calls `sudo`, a
system package manager, or an installer.

All maintained local project operations use `mise run <task>`. Legacy
direct-execution examples are temporary migration debt owned by
`08-07-migrate-docs-and-trellis-specs` and are explicitly allowlisted by
`docs-contract-check.mjs`; new occurrences fail the contract. CI and Release
remain the explicit non-mise execution boundary.

## 5. Lock and Update Governance

Normal bootstrap/install consumes existing locks and never bumps them. An
intentional full lock regeneration is:

```bash
mise lock --platform linux-x64,linux-arm64,macos-x64,macos-arm64,windows-x64,windows-arm64
mise run tasks:validate
```

Generate the lock from an empty-file state when changing a backend alias, then
run it a second time and require byte stability. Do not hand-edit a checksum,
URL, backend, platform key, or generated marker.

Toolchain and dependency update tasks are ecosystem-specific and preview by
default. They require `--apply` before writing; no task commits, tags, pushes,
changes remotes, opens a PR, or publishes. A failed toolchain update restores
the standard version file and `mise.lock` captured before the attempt.

## 6. Validation / Error Matrix

| Condition                                                      | Required result                                                                         |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| mise is missing or older than 2026.8.0                         | Stop before dependency preparation                                                      |
| Ordinary task is started with a missing tool                   | Fail and direct the developer to `bootstrap`; never auto-trust                          |
| A standard version differs from the actual executable          | `env:check` fails                                                                       |
| `mise.toml` repeats Node/pnpm/Rust/Python                      | Lock and environment contracts fail                                                     |
| Python resolves outside uv management or `.venv` is absent     | Python/environment checks fail                                                          |
| WSL resolves a managed executable below `/mnt/<drive>`         | Fail and repair PATH; never invoke the Windows shim                                     |
| Lock platform URL names another architecture                   | Fail, even when checksum and URL are otherwise valid                                    |
| Rust lock has no platform assets                               | Record exact version/options plus native rustup evidence; never invent an asset claim   |
| A script changes mise trust or private mise/Cargo/rustup homes | Reject the change                                                                       |
| A standard dev/build/test task accepts `--target`              | Reject before Cargo/Tauri execution                                                     |
| Host native libraries are missing                              | `system:check` fails with a non-elevating installation hint                             |
| A prerequisite command is absent or cannot be launched         | Record a failed check with its installation hint and finish the machine-readable report |

## 7. Tests Required

- Parse every standard source and assert Node 24.19.0, pnpm 10.12.3, Rust
  1.97.1, and Python 3.14.7 without duplicate mise declarations.
- Regenerate `mise.lock` from no prior lock, target all six platforms, and
  require an identical second generation.
- Structurally validate backend identity, URLs, SHA-256 checksums, platform
  architecture, native Windows ARM64 pnpm/uv assets, Rust options, and absence
  of mise-managed Python, release targets, and `llvm-tools`.
- Run `uv lock --check --offline`, `uv sync --locked`, and locked/no-sync/offline
  Python 3.14.7 through the created `.venv`.
- Run `mise config ls --json`, `mise tasks ls --json`, `env:check --json`, and
  current-host `system:check`; path comparisons must work with native Windows
  separators, and an empty PATH probe must still return the complete JSON
  failure report with a hint for every missing prerequisite.
- Verify Node/pnpm/uv resolve to `mise which`, and prove Rust with
  `mise which rustc`, the exact rustup active toolchain, components, and sysroot.
- Exercise a parameter plus flag through real `mise run`, and prove filters
  cannot smuggle `--target` into Rust tests.
- Run `developmentEnvironment.test.ts`, `miseTaskContract.test.ts`,
  `taskDocs.test.ts`, `systemCheck.test.ts`, and `localBuildBoundary.test.ts`.
- Obtain native Windows ARM64, Linux ARM64, Windows x64, macOS, and Linux x64
  runner evidence before claiming all supported platforms verified. Local
  Linux success is not substitute evidence for another OS/architecture.

## 8. Wrong vs Correct

Wrong: duplicate versions in mise, accept an x64 URL under an ARM64 key, use a
system Python fallback, run a repository trust task, silently install system
packages, or call a current-host build with a foreign target.

Correct: standard ecosystem files select exact versions, mise orchestrates and
locks audited assets, uv owns Python, canonical tasks make side effects
explicit, and native runners close every platform-specific evidence gate.
