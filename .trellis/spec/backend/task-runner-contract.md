# Repository Task Runner Contract

## 1. Scope / Trigger

Read this contract before adding, renaming, removing, documenting, or composing
a `mise run` task or changing `scripts/tasks/`. The task API is the stable local
entrypoint for developers, Trellis, and Codex hooks. Package scripts and Cargo
commands remain implementation leaves; GitHub Actions is an explicit non-mise
boundary and the sole executor for non-host platform work.

## 2. Layout and Signatures

`mise.toml` explicitly includes these domain files:

```text
.mise/tasks/core.toml
.mise/tasks/frontend.toml
.mise/tasks/rust.toml
.mise/tasks/python.toml
.mise/tasks/trellis.toml
.mise/tasks/upstream.toml
.mise/tasks/contracts.toml
.mise/tasks/release.toml
.mise/tasks/hooks.toml
```

Included TOMLs use mise's task-file format (top-level task tables, no
`[tasks]` prefix). Simple leaves wrap pnpm/Cargo/uv directly. Complex,
parameterized, filesystem, Git, environment, lock, documentation, or
maintenance logic lives in cross-platform Node `.mjs` scripts; core task
behavior may not depend on Bash.

Every public task has:

- a non-empty `description`;
- `env.FYAGENT_TASK_EFFECT` from the approved effect vocabulary;
- a formal `usage` declaration whenever it accepts an argument or flag;
- `interactive = true`, `raw = true`, or an explicit confirmation only when
  its I/O contract requires that behavior.

The v0.3.0 baseline contains the eighty tasks generated in
`docs/fyagent/development/mise-tasks.md`. Later requirements may add tasks when
they satisfy this contract; validation requires the canonical baseline as a
subset instead of forbidding safe extensions.

## 3. Composition and Side Effects

`check` executes `env:check`, frontend, backend, and contracts. Its complete
task-reference closure must have effect `read-only`. Mutation, dependency
installation, build output, interactive tasks, temporary dependency tools,
Trellis writes, Git ref writes, and preview-by-default maintenance tasks never
enter that closure.

`check:backend` uses structured sequential task references in this order:

```text
rust:fmt:check -> rust:check -> rust:clippy -> rust:test
```

Frontend checks may be extended by later contracts without replacing the
stable `check`/`check:frontend` entrypoints. A task must not reference a
nonexistent future test or claim a domain gate before that domain implements
it.

`dev`, `build`, `build:binary`, and `build:debug` are fixed current-host
operations and have no caller argument. `pnpm dev`/`pnpm build` and those mise
tasks route through one shared Node wrapper. The wrapper validates the exact
process OS/architecture against matching absolute `rustc`/`rustdoc` `-vV`
identities. Before probing or launching a toolchain it rejects caller target,
compiler, rustdoc, wrapper, or target-runner/linker controls case-insensitively and
rejects target-bearing ordinary/build/encoded flag sources plus every
target-specific Rust/rustdoc flag source. Process loader/runtime injection
controls are rejected before probing and cleared from toolchain children.
The child owns the absolute tools, empty wrapper/flag settings, and explicit
current-host target. Before starting the toolchain, the wrapper recursively
inspects every effective Cargo config and rejects build target/compiler/
rustdoc/wrapper/flags plus target runner/linker/flags, including config include
cycles and symlinks. The same protected-name classifier rejects corresponding
Cargo config `[env]` keys regardless of case or string/table value. Cargo test
receives its native direct runner through a
CLI TOML argv array built from the current Node process and same wrapper; no
shell quoting is involved. The runner validates target/path/file/native format
and exact ELF/PE/Mach-O machine identity, then directly spawns the test binary
with `shell: false`; filters remain argv and never enter a shell.
`rust:check`, `rust:clippy`, and `rust:test` use the same guard; rustfmt does not need a target. `rust:test` accepts at most one test-name
filter, passes it after Cargo's `--`, and rejects every option-like value; in
particular, a caller cannot smuggle `--target` through a variadic usage field.

The lower-level `pnpm tauri` package leaf is retained for reviewed Actions and
maintenance commands that deliberately do not use the local task API. It is
not a standard local entrypoint. This contract enforces canonical wrappers; it
does not claim to intercept an arbitrary hand-written Cargo/Tauri command, and
such a command cannot provide project acceptance evidence.

Portable policy tests can run on the current host, but their result remains a
portable contract result. Windows, macOS, ARM64, and any other non-host native
gate runs only on its matching GitHub Actions runner. Repository tasks never
install or activate a non-host Rust target as part of local execution.

## 4. Parameter Transport

mise parses each `usage` spec and exports `usage_<name>` values. Node wrappers
read those values, parse variadic shell-escaped lists into argv arrays, validate
SemVer/package/tag/enum/path inputs, and spawn a command without a shell.
Arguments must never be concatenated into a command string.

On native Windows, local mise tasks resolve only the actually used `pnpm`
command to `pnpm.exe`. This matches the audited `mise.lock` assets
`pnpm-win-x64.exe` and `pnpm-win-arm64.exe`; both carry required SHA-256
checksums. The task runner does not synthesize `.cmd` names for pnpm, npm, npx,
or pnpx and does not introduce `cmd.exe`, `shell: true`, or command-string
quoting. Non-Windows commands remain direct. This local mise boundary is
distinct from GitHub Actions, which does not install mise and uses its own
reviewed `pnpm.cmd` batch-shim bridge in the CI toolchain verifier.

Contract tests execute real `mise run` calls for a positional value, a flag,
and a filtered test. Metadata inspection alone is not sufficient proof that
values reach the wrapper.

## 5. Mutation Policies

- `bootstrap` may install locked tools/dependencies but may not trust, install
  system packages, change Git, refresh locks, build, or publish.
- Formatting is an explicit source-modifying leaf and does not prompt.
- Version, dependency, toolchain, Python lock/dependency, icon, task-doc, and
  clean tasks preview by default; `--apply` is required to write.
- `version:set` and `version:bump` delegate to the canonical atomic version
  tool and remain dry-run by default.
- Clean tasks select only an internal allowlist, resolve every target below the
  repository root, and never delete locks, `.git`, `.trellis`, baselines, or
  end-user data.
- `upstream:fetch` fetches one validated tag. Merge preparation requires a
  clean worktree and `--apply`, and may only enter
  `git merge --no-ff --no-commit`. Upstream tasks never change remotes, resolve
  conflicts, commit, tag, or push.
- `release:check` is read-only; no local task signs, uploads, creates, edits, or
  deletes a GitHub Release.
- No local task compiles, packages, or verifies a non-host OS/architecture.
  Release helpers may be referenced by matching native Actions jobs, but no
  local alias or wrapper turns them into a cross-platform acceptance path.

## 6. Generated Documentation

`task-docs.mjs` reads the actual included TOML metadata. It escapes Markdown
pipe characters, emits every loaded task, and writes only when
`tasks:docs:generate --apply` is used. `tasks:docs:check` regenerates in memory
and byte-compares with the committed document.

Active docs that still use the retired direct-execution project-entrypoint style
are an explicit Child 6 handoff allowlist. A new legacy occurrence fails
`docs-contract-check.mjs`; removing an allowlisted occurrence is always safe.
Retired local cross-build tasks have no alias or deprecation forwarder.

## 7. Validation / Error Matrix

| Condition                                                             | Required result                        |
| --------------------------------------------------------------------- | -------------------------------------- |
| Missing description/effect/usage                                      | `tasks:validate` fails                 |
| Missing task reference or DAG cycle                                   | mise/task contract fails               |
| `check` reaches a non-read-only effect                                | Fail closed                            |
| A parameter is interpolated into a shell command                      | Reject; spawn validated argv instead   |
| A Windows task forces a pnpm batch shim instead of locked `pnpm.exe`  | Task-runner and DEP0040 contracts fail |
| A Rust filter begins with `-` or contains `--target`                  | Reject before rustc or Cargo starts    |
| A fixed native operation receives forwarded argv                      | Reject before rustc or Tauri starts    |
| Caller compiler/wrapper/runner/linker/target env redirects a task     | Reject before rustc/rustdoc starts     |
| Any Rust/rustdoc flag env contains a target token                     | Reject before rustc/rustdoc starts     |
| Target-specific flags or process-loader/runtime injection are set     | Reject before rustc/rustdoc starts     |
| Absolute rustc/rustdoc identity and process host disagree             | Reject before Cargo/Tauri starts       |
| User Cargo config selects target/compiler/wrapper/flags/runner/linker | Reject before the toolchain starts     |
| A standard task selects a non-host OS/architecture                    | Reject before any toolchain starts     |
| A local wrapper bridges to a foreign executable/emulator              | Reject; require a native Actions job   |
| Mutation task has neither preview default nor explicit confirmation   | Reject                                 |
| Clean path resolves outside the repository                            | Reject without deletion                |
| Upstream safety/remotes/worktree do not match                         | Reject before fetch/merge              |
| Generated task reference differs by one byte                          | `tasks:docs:check` fails               |
| New active doc uses a legacy entrypoint                               | `docs-contract-check.mjs` fails        |

## 8. Tests Required

- `mise tasks validate --errors-only` and `task-contract-check.mjs`.
- Required-task subset, metadata/effect/usage, reference closure, check DAG,
  Rust order, retired task, and forbidden command scans.
- Real parameter/flag transport smoke tests, including dry-run `version:set`,
  a test filter, Python preview input, and upstream tag validation.
- Pure executable-resolution tests must require `pnpm.exe` only on Win32,
  preserve direct non-Windows commands, bind both native Windows pnpm lock
  assets and checksums, and prove the DEP0040 checker uses the shared resolver
  without a `pnpm.cmd` fallback.
- Pure tests for all six supported process-host mappings, strict absolute
  rustc/rustdoc identity, case-insensitive caller compiler/wrapper/runner/linker/
  target and target-bearing flag rejection, plus fixed current-host
  Tauri/Cargo argv and owned child environment.
- Encode Node runner argv as a Cargo CLI TOML array without a shell, including
  absolute paths containing whitespace; validate its current-target repository
  boundary, non-symlink native format and exact machine identity, and direct
  argv execution with current-host-only fixtures and metacharacter filters.
- Recursively scan effective Cargo config sources and includes; reject
  compiler/wrapper/flags/runner/linker controls, symlinks, and include cycles
  before any toolchain process starts.
- Negative Rust `--target` smuggling tests through normal and double-dash
  invocation paths, plus real `pnpm`/mise wrapper smoke proving rejection
  occurs before rustc, Cargo, or Tauri starts.
- Require the aggregate `check` task to begin with the subprocess-free
  host-native guard before `env:check`, so caller compiler/runner/target
  controls cannot reach its rustc toolchain probe.
- Active local-entrypoint scans must cover package scripts and all included
  task TOMLs, reject cross-target/cross-tool execution markers, and leave
  `.github/workflows/**` outside that negative set so native runner targets
  remain required and testable.
- Clean preview tests proving canonical repository-only targets and zero writes.
- Docs generation/check tests including a description containing `|` to prove
  table escaping.
- `developmentEnvironment.test.ts`, `miseTaskContract.test.ts`,
  `taskDocs.test.ts`, `systemCheck.test.ts`, and
  `localBuildBoundary.test.ts`.

## 9. Wrong vs Correct

Wrong: put every command back into one `mise.toml`, rely on Bash, infer safety
from a task name, concatenate usage input, let check install/update, bypass the
wrapper with a low-level local target command, or hand-edit generated task
rows.

Correct: domain TOMLs describe a stable API, Node wrappers validate boundaries,
effects make composition auditable, guarded native wrappers verify and pin the
current host, and executable tests prove both metadata and real argument flow.
