# Repository Task Runner Contract

## Canonical interface

All local repository operations use the full form:

```text
mise run <canonical-task>
```

Exceptions are mise lifecycle commands (`mise trust`, `mise install`, `mise lock`, `mise tasks`), GitHub Actions (which does not install mise), and operating-system package managers used for Tauri prerequisites.

Task implementations may directly call tools because mise has already selected the environment. A task MUST NOT recursively wrap every command in another `mise run`.

## Organization

```text
mise.toml                  tool declarations, settings, includes
.mise/tasks/*.toml         small domain task definitions
scripts/tasks/*.mjs        complex, testable, cross-platform implementations
.trellis/scripts/*.py      Trellis state machine, invoked through uv wrappers
```

Cross-platform core behavior uses Node `.mjs`, not Bash. Platform-specific shell is allowed only when the task is explicitly limited to that platform.

## Public namespaces

```text
bootstrap, env:check, system:check, deps:install
dev*, build*
check*, typecheck, format*, test*
rust*, python*, trellis*, codex:hook*
version*, assets*, clean*
deps:*, toolchain:*
upstream:*, release:check, tasks:*
```

Every public task has a description. Tasks with parameters have a `usage` contract. Interactive tasks are marked interactive. Mutation tasks are not dependencies of `bootstrap`, `test`, `check`, or CI.

## Build boundary

`dev` and `build*` operate only on the current host OS and architecture. Standard tasks reject arbitrary cross-OS `--target` values. The following retired names MUST NOT exist or be aliased:

```text
macos:preflight
build:cross-windows:x64
build:cross-windows:arm64
build:cross-windows
build:cross-macos:universal
```

Local bundle output is development evidence, not a formal release asset.

## Side effects and safety

- `version:set` and `version:bump` default to dry-run; `--apply` is required.
- Whole-ecosystem dependency updates require `--all` and confirmation.
- `assets:*`, visual-baseline updates, lock bumps, cleanup, and upstream merge preparation are explicitly mutating.
- Clean tasks validate that every target is inside the repository and never delete manifests, locks, Git/Trellis state, visual baselines, or user data.
- FyAgent build/version/update/upstream/release tasks do not auto-commit, tag, push, create a Release, rewrite remotes, or grant trust. The existing Trellis archive state machine may create its documented archive commit; bulk superseded-task archival uses `--no-commit` and one explicit reviewed commit.

## Composition

`bootstrap` runs sequentially. Rust fmt/check/clippy/test run sequentially to avoid target-lock contention. Independent frontend checks may run in parallel. `check:frontend`, `check:backend`, and `check:contracts` may run in parallel while preserving internal ordering. Watch/dev tasks are interactive and never appear in a check dependency graph.

## Documentation and compatibility

Task documentation is generated from metadata and checked by regeneration. Current project docs use canonical names only. Ordinary task renames keep a documented compatibility entry for at least one FyAgent release cycle; retired cross-build tasks receive no alias.

## Tests Required

- `mise tasks validate --errors-only` plus project contract checks;
- unique names, descriptions, usage, platform/interaction metadata, and acyclic DAG;
- package scripts either mapped or explicitly allowlisted;
- no mutation task reachable from read-only aggregates;
- generated task-doc comparison;
- active-document scan for nonexistent/deprecated task names and direct project tool invocations.
