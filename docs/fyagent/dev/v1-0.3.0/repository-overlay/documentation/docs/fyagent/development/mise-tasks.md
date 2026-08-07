# Proposed FyAgent mise Task Catalog

> **Status: proposed catalog.** This file is a design-time reference, not output
> from an implemented `mise generate task-docs` command. After tasks exist in the
> repository, regenerate this document from task metadata and require
> `mise run tasks:docs:check` in CI.

Use the full form `mise run <task>`. GitHub Actions is the explicit non-mise
execution boundary.

## Setup and Checks

| Task | Purpose | Side effect |
| --- | --- | --- |
| `bootstrap` | Prepare tools/dependencies, uv environment, strict checks | installs declared tools/dependencies; no lock update |
| `env:check` | Strict tool/version/source/lock/hook verification | read-only |
| `system:check` | Strict host Tauri prerequisite report | read-only |
| `deps:install` | Frozen pnpm install + `uv sync --locked` | dependency environment only |
| `check` | Current-host frontend/backend/contracts aggregate | read-only |
| `check:frontend` | Type, format, unit/i18n/desktop mock/preflight | read-only |
| `check:backend` | Rust fmt/check/clippy/test in controlled order | read-only |
| `check:contracts` | Toolchain/task/docs/workflow/release/DEP0040 contracts | read-only |

## Development and Native Build

| Task | Purpose | Notes |
| --- | --- | --- |
| `dev` | Tauri desktop development | interactive; current host |
| `dev:renderer` | Vite renderer only | interactive |
| `build:renderer` | production renderer | no desktop package |
| `build:binary` | Tauri release binary, `--no-bundle` | current host only |
| `build` | native current-host bundle | not a formal Release asset |
| `build:debug` | current-host debug build/bundle | diagnostic |
| `release:check` | local read-only release contract checks | no signing/upload/release |

Standard tasks do not accept cross-OS `--target`. The retired `build:cross-*`
and `macos:preflight` tasks have no aliases.

## Frontend and Desktop Tests

| Task | Purpose | Behavior |
| --- | --- | --- |
| `typecheck` | strict TypeScript check | read-only |
| `format` | apply formatter | modifies files |
| `format:check` | verify formatting | read-only |
| `test` | non-interactive frontend/desktop test aggregate | read-only |
| `test:unit [filter...]` | Vitest unit/integration tests | controlled filter |
| `test:unit:watch [filter...]` | Vitest watch | interactive |
| `test:i18n` | locale key/schema parity | read-only |
| `test:desktop:mock` | desktop/Tauri fake contract | not native evidence |
| `test:desktop:visual:preflight` | validate candidate/baseline contract | read-only |
| `test:desktop:visual:update <evidence>` | update reviewed baseline | modifies; confirm |

## Rust

| Task | Purpose |
| --- | --- |
| `rust:fmt` | apply Cargo fmt |
| `rust:fmt:check` | verify Cargo fmt |
| `rust:check` | workspace/all-targets `cargo check --locked` |
| `rust:clippy` | workspace/all-targets Clippy with `-D warnings` |
| `rust:test [filter...]` | locked workspace tests with controlled filter |

On Windows, dev/build tasks use `FYAGENT_WINDOWS_MANIFEST=dev`; check/test uses
`test`; only formal Release Actions may use `release`.

## Python and uv

| Task | Purpose |
| --- | --- |
| `python:sync` | `uv sync --locked` |
| `python:lock` | intentionally refresh `uv.lock` |
| `python:lock:check` | verify lock freshness |
| `python:check` | uv/Python/.venv/project contract |
| `python:add:dev <requirements...>` | add repeatable dev dependencies |
| `python:remove:dev <packages...>` | remove repeatable dev dependencies |
| `python:update <packages...>` | targeted lock upgrade |
| `python:with <requirement> -- <command>` | isolated one-off dependency |
| `python:tool -- <tool> [args]` | isolated `uv tool run` / `uvx` |
| `python:run -- <command>` | locked project Python command |

Python is not declared in mise; uv uses `.python-version` and managed-only
Python to create `.venv`.

## Trellis and Codex Hooks

| Task | Purpose |
| --- | --- |
| `trellis:init-developer <name>` | initialize developer identity |
| `trellis:get-developer` | read current identity |
| `trellis:context [args]` | workflow/package/phase context |
| `trellis:task -- <subcommand> [args]` | thin wrapper around task.py |
| `trellis:session:add -- [args]` | record completed work |
| `trellis:validate [task]` | validate Trellis task manifests |
| `codex:hook:workflow-state` | offline/no-sync prompt context hook |
| `codex:hook:subagent-context` | offline/no-sync subagent context hook |
| `codex:hooks:check` | simulate hooks and verify protocol/no side effects |

## Version, Assets, Cleanup

| Task | Purpose | Safety |
| --- | --- | --- |
| `version:get` | print product version | read-only |
| `version:check [--tag vX.Y.Z]` | verify version contract | read-only |
| `version:set X.Y.Z [--apply]` | dry-run by default | modifies only with `--apply` |
| `version:bump patch|minor|major [--apply]` | dry-run by default | modifies only with `--apply` |
| `assets:icons [--source file] [--apply]` | preview/generate icon set | confirm before modifying |
| `assets:icons:check` | decode/size/consumer validation | read-only |
| `clean:frontend` | remove frontend generated state | confirm scope |
| `clean:rust` | remove Cargo target output | confirm scope |
| `clean:python` | remove project `.venv` | confirm scope |
| `clean:artifacts` | remove local bundles/release-assets | confirm scope |
| `clean:all` | controlled aggregate | explicit confirmation |

Clean tasks validate every deletion path is inside the repository and never
remove locks, Git/Trellis state, historical baselines, or end-user data.

## Dependency and Toolchain Maintenance

| Task | Purpose |
| --- | --- |
| `deps:outdated` | aggregate read-only reports |
| `deps:outdated:frontend` | pnpm outdated report |
| `deps:outdated:rust` | Cargo dry-run report |
| `deps:outdated:python` | uv outdated report |
| `deps:update:frontend <packages...> [--all]` | targeted or explicit full pnpm update |
| `deps:update:rust <crates...> [--all]` | targeted or explicit full Cargo update |
| `toolchain:outdated` | report candidate Node/Rust/pnpm/uv/Actions updates |
| `toolchain:update:node X.Y.Z` | update `.node-version`, locks, checks |
| `toolchain:update:rust X.Y.Z` | update rust toolchain, locks, checks |
| `toolchain:update:pnpm X.Y.Z` | update packageManager, locks, checks |
| `toolchain:update:uv` | controlled `mise lock --bump uv` |
| `toolchain:lock` | regenerate/validate supported-platform mise lock |

There is no implicit cross-ecosystem `update all`; no maintenance task commits,
tags, pushes, opens, or auto-merges a pull request.

## Upstream

| Task | Purpose |
| --- | --- |
| `upstream:check` | remote URL/push-disable/worktree/baseline safety |
| `upstream:fetch <tag>` | fetch one validated upstream tag only |
| `upstream:audit <tag>` | tag SHA, merge base, commits, diff classification |
| `upstream:merge:prepare <tag>` | confirm, then `--no-ff --no-commit` merge |
| `upstream:merge:abort` | abort only a valid active merge |

No upstream task resolves conflicts, creates a merge commit, changes remotes,
tags, force-pushes, or pushes to origin/upstream.

## Task Metadata and Documentation

| Task | Purpose |
| --- | --- |
| `tasks:validate` | task metadata/DAG/script/docs contract |
| `tasks:docs:generate` | generate canonical task reference from metadata |
| `tasks:docs:check` | regenerate in temp and compare |

Every public task has a description; parameterized tasks have a structured
usage contract. Normal renames keep a one-release deprecation forwarder, except
retired cross-build tasks.
