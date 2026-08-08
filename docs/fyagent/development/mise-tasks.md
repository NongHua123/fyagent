# FyAgent mise Task Reference

> Generated from `.mise/tasks/*.toml` by `mise run tasks:docs:generate --apply`.
> Do not edit task rows by hand; `mise run tasks:docs:check` performs a byte comparison.

Use `mise run <task>`. GitHub Actions is the explicit non-mise execution boundary.
Parameterized tasks expose their contract through `mise run <task> --help`.
Tasks marked preview-by-default require `--apply` before they write or delete repository state.

Standard versions: Node 24.19.0, pnpm 10.12.3, Rust 1.97.1, Python 3.14.7;
the approved `uv = latest` resolution is pinned in `mise.lock`.

## Setup and Checks

| Task              | Description                                                                             | Usage  | Effect                 |
| ----------------- | --------------------------------------------------------------------------------------- | ------ | ---------------------- |
| `bootstrap`       | Install locked repository tools and dependencies, then run strict environment checks    | —      | dependency-environment |
| `check`           | Run the complete current-host environment, frontend, backend, and contract gate         | —      | read-only              |
| `check:backend`   | Run Rust formatting, check, Clippy, and tests in fail-fast order                        | —      | read-only              |
| `check:contracts` | Run task, docs, Python lock, version, hook, and release contract checks                 | —      | read-only              |
| `check:frontend`  | Run frontend type, formatting, unit, i18n, desktop mock, and visual preflight checks    | —      | read-only              |
| `deps:install`    | Install frozen pnpm dependencies and synchronize the locked uv environment              | —      | dependency-environment |
| `env:check`       | Verify exact tools, ownership, lockfiles, Python environment, tasks, and hook contracts | --json | read-only              |
| `system:check`    | Check current-host Tauri prerequisites without installing or elevating anything         | --json | read-only              |

## Development and Native Build

| Task             | Description                                                               | Usage | Effect       |
| ---------------- | ------------------------------------------------------------------------- | ----- | ------------ |
| `build`          | Build the current host's native Tauri bundle (not a formal Release asset) | —     | build-output |
| `build:binary`   | Build the current host's release Tauri binary without a bundle            | —     | build-output |
| `build:debug`    | Build a debug Tauri bundle for the current host                           | —     | build-output |
| `build:renderer` | Build the production renderer without a desktop package                   | —     | build-output |
| `dev`            | Start native Tauri development on the current host                        | —     | interactive  |
| `dev:renderer`   | Start the Vite renderer development server                                | —     | interactive  |

## Frontend and Desktop Tests

| Task                            | Description                                                                    | Usage      | Effect           |
| ------------------------------- | ------------------------------------------------------------------------------ | ---------- | ---------------- |
| `format`                        | Apply the repository Prettier configuration to frontend sources                | —          | source-modifying |
| `format:check`                  | Verify frontend formatting without writing files                               | —          | read-only        |
| `test`                          | Run the non-interactive frontend and desktop mock test aggregate               | —          | read-only        |
| `test:desktop:mock`             | Run desktop acceptance fake-IPC contract tests (not native evidence)           | —          | read-only        |
| `test:desktop:visual:preflight` | Validate visual baseline manifest and candidate policy without updating images | —          | read-only        |
| `test:desktop:visual:update`    | Validate reviewed visual evidence before a separate human Git LFS update       | <evidence> | source-modifying |
| `test:i18n`                     | Verify locale key and schema parity                                            | —          | read-only        |
| `test:unit`                     | Run Vitest unit and integration tests with optional controlled filters         | [filters]  | read-only        |
| `test:unit:watch`               | Run Vitest in interactive watch mode with optional controlled filters          | [filters]  | interactive      |
| `typecheck`                     | Run strict TypeScript type checking without emitting files                     | —          | read-only        |

## Rust

| Task             | Description                                                             | Usage     | Effect           |
| ---------------- | ----------------------------------------------------------------------- | --------- | ---------------- |
| `rust:check`     | Run locked Cargo check for every target kind on the current host        | —         | read-only        |
| `rust:clippy`    | Run locked current-host Clippy for the workspace and deny every warning | —         | read-only        |
| `rust:fmt`       | Apply rustfmt to the complete Cargo workspace                           | —         | source-modifying |
| `rust:fmt:check` | Verify rustfmt for the complete Cargo workspace                         | —         | read-only        |
| `rust:test`      | Run locked current-host Cargo tests with optional controlled filters    | [filters] | read-only        |

## Python and uv

| Task                | Description                                                                      | Usage                   | Effect                 |
| ------------------- | -------------------------------------------------------------------------------- | ----------------------- | ---------------------- |
| `python:add:dev`    | Preview or add repeatable development dependencies to pyproject.toml and uv.lock | <requirements> --apply  | preview-by-default     |
| `python:check`      | Verify uv ownership, Python version, .venv, pyproject, and uv.lock contracts     | —                       | read-only              |
| `python:lock`       | Preview or intentionally refresh uv.lock                                         | --apply                 | preview-by-default     |
| `python:lock:check` | Verify uv.lock is current without synchronizing or accessing the network         | —                       | read-only              |
| `python:remove:dev` | Preview or remove repeatable development dependencies                            | <packages> --apply      | preview-by-default     |
| `python:run`        | Run a command inside the locked uv project environment                           | <command>               | user-command           |
| `python:sync`       | Synchronize the uv-managed Python environment from uv.lock                       | —                       | dependency-environment |
| `python:tool`       | Run an isolated Python CLI through uv tool run                                   | <command>               | user-command           |
| `python:update`     | Preview or apply a targeted uv dependency lock upgrade                           | <packages> --apply      | preview-by-default     |
| `python:with`       | Run a command with one isolated temporary Python requirement                     | <requirement> <command> | user-command           |

## Trellis and Codex Hooks

| Task                          | Description                                                                          | Usage  | Effect        |
| ----------------------------- | ------------------------------------------------------------------------------------ | ------ | ------------- |
| `codex:hook:subagent-context` | Inject curated Trellis context into a native Codex subagent without syncing Python   | —      | read-only     |
| `codex:hook:workflow-state`   | Inject Trellis workflow state into a Codex prompt without syncing Python             | —      | read-only     |
| `codex:hooks:check`           | Verify Codex hook protocol, integrity, offline execution, and side-effect boundaries | —      | read-only     |
| `trellis:context`             | Read Trellis workflow, package, phase, or record context                             | [args] | read-only     |
| `trellis:get-developer`       | Read the current Trellis developer identity                                          | —      | read-only     |
| `trellis:init-developer`      | Initialize the Trellis developer identity through the uv-managed Python environment  | <name> | trellis-state |
| `trellis:session:add`         | Record a Trellis session journal entry through the uv-managed Python environment     | <args> | trellis-state |
| `trellis:task`                | Run a Trellis task.py subcommand through the uv-managed Python environment           | <args> | trellis-state |
| `trellis:validate`            | Validate one Trellis task or every active Trellis task manifest                      | [task] | read-only     |

## Version, Assets, and Cleanup

| Task                 | Description                                                                       | Usage                   | Effect             |
| -------------------- | --------------------------------------------------------------------------------- | ----------------------- | ------------------ |
| `assets:icons`       | Preview or generate the application icon set from a validated source image        | --source <file> --apply | preview-by-default |
| `assets:icons:check` | Verify required application icon consumers and basic file signatures              | —                       | read-only          |
| `clean:all`          | Preview or remove every approved repository-local generated directory             | --apply                 | preview-by-default |
| `clean:artifacts`    | Preview or remove repository-local package and release artifacts                  | --apply                 | preview-by-default |
| `clean:frontend`     | Preview or remove repository-local frontend generated state                       | --apply                 | preview-by-default |
| `clean:python`       | Preview or remove the repository-local uv .venv                                   | --apply                 | preview-by-default |
| `clean:rust`         | Preview or remove repository-local Cargo target output                            | --apply                 | preview-by-default |
| `version:bump`       | Preview or atomically bump the product version through the canonical version tool | <level> --apply         | preview-by-default |
| `version:check`      | Verify every product-version consumer and an optional release tag                 | --tag <tag>             | read-only          |
| `version:get`        | Print the Cargo-workspace product version                                         | —                       | read-only          |
| `version:set`        | Preview or atomically set the product version through the canonical version tool  | <version> --apply       | preview-by-default |

## Dependency and Toolchain Maintenance

| Task                     | Description                                                                    | Usage                    | Effect             |
| ------------------------ | ------------------------------------------------------------------------------ | ------------------------ | ------------------ |
| `deps:outdated`          | Report outdated frontend, Rust, and Python dependencies without changing locks | —                        | read-only          |
| `deps:outdated:frontend` | Report outdated pnpm dependencies                                              | —                        | read-only          |
| `deps:outdated:python`   | Report outdated uv project dependencies                                        | —                        | read-only          |
| `deps:outdated:rust`     | Report Cargo lock update candidates with a dry run                             | —                        | read-only          |
| `deps:update:frontend`   | Preview or apply a targeted (or explicit all-package) pnpm update              | [packages] --all --apply | preview-by-default |
| `deps:update:rust`       | Preview or apply a targeted (or explicit all-crate) Cargo lock update          | [crates] --all --apply   | preview-by-default |
| `toolchain:lock`         | Preview or regenerate mise.lock for every supported platform                   | --apply                  | preview-by-default |
| `toolchain:outdated`     | Report candidate Node, Rust, pnpm, and uv toolchain updates                    | —                        | read-only          |
| `toolchain:update:node`  | Preview or apply an exact Node version-file and mise-lock update               | <version> --apply        | preview-by-default |
| `toolchain:update:pnpm`  | Preview or apply an exact packageManager and mise-lock update                  | <version> --apply        | preview-by-default |
| `toolchain:update:rust`  | Preview or apply an exact Rust toolchain and mise-lock update                  | <version> --apply        | preview-by-default |
| `toolchain:update:uv`    | Preview or apply a controlled uv latest-selector lock bump                     | --apply                  | preview-by-default |

## Upstream

| Task                     | Description                                                                        | Usage         | Effect             |
| ------------------------ | ---------------------------------------------------------------------------------- | ------------- | ------------------ |
| `upstream:audit`         | Report upstream tag object, peeled commit, merge base, commits, and diff summary   | <tag>         | read-only          |
| `upstream:check`         | Verify immutable upstream identity, push-disable, worktree, and merge-state safety | —             | read-only          |
| `upstream:fetch`         | Fetch one validated upstream tag without changing remotes or other tags            | <tag>         | git-fetch          |
| `upstream:merge:abort`   | Abort only an active Git merge after explicit confirmation                         | —             | git-state          |
| `upstream:merge:prepare` | Preview or enter an uncommitted two-parent upstream merge state                    | <tag> --apply | preview-by-default |

## Task Metadata and Documentation

| Task                  | Description                                                                                      | Usage   | Effect             |
| --------------------- | ------------------------------------------------------------------------------------------------ | ------- | ------------------ |
| `tasks:docs:check`    | Regenerate task documentation in memory and byte-compare it with the committed reference         | —       | read-only          |
| `tasks:docs:generate` | Preview or regenerate the canonical mise task reference from task metadata                       | --apply | preview-by-default |
| `tasks:validate`      | Validate task metadata, DAG, safety classes, scripts, locks, and active-doc migration boundaries | —       | read-only          |

## Release Contract

| Task            | Description                                                                              | Usage | Effect    |
| --------------- | ---------------------------------------------------------------------------------------- | ----- | --------- |
| `release:check` | Run local read-only release contracts without tagging, signing, uploading, or publishing | —     | read-only |

## Safety Boundaries

- `bootstrap` never changes trust, system packages, Git remotes, locks, tags, or releases.
- `check` reaches read-only tasks only; Rust checks remain ordered fmt → check → Clippy → test.
- `dev` and `build*` are current-host only and expose no cross-OS or cross-architecture target.
- Clean tasks canonicalize an allowlisted repository child path and never remove Git, Trellis, locks, historical baselines, or user data.
- Upstream tasks do not change remotes, resolve conflicts, commit, tag, or push.
- `release:check` is read-only; only GitHub Actions may publish a formal release.
