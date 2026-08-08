# GitHub CI, Required Gate, and Labeler Contract

## 1. Scope / Trigger

This contract applies to `.github/workflows/ci.yml`,
`.github/workflows/labeler.yml`, `.github/labeler.yml`, and the pure-Node CI
helpers under `scripts/ci/`. Read it before changing an automatic CI event,
required job, runner label, toolchain setup, Action reference, workflow token
permission, or pull-request labeling behavior.

The CI workflow runs for all of these events without a workflow-level path
filter:

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  merge_group:
    types: [checks_requested]
  workflow_dispatch:
```

The Labeler runs automatically from the trusted base context and retains an
explicit numeric manual replay:

```yaml
on:
  pull_request_target:
    branches: [main]
  workflow_dispatch:
    inputs:
      pr_number:
        required: true
        type: number
```

Local static validation does not prove that GitHub accepted, scheduled, or
completed any of these remote events. Local execution is host-native only;
every non-host native CI result comes from its matching Actions runner.

## 2. Signatures

### Required topology

```text
contracts        (ubuntu-24.04) \
frontend         (ubuntu-24.04)  \
desktop-acceptance-contract (ubuntu-24.04) +--> CI / Required (ubuntu-24.04, always)
backend-linux    (ubuntu-24.04)   /
backend-windows  (windows-2022)  /
windows-msi-query (windows-2022 x64 + windows-11-arm ARM64) /
backend-macos    (macos-15)     /
```

The seven dependency job IDs are an exact machine-readable contract:

```text
contracts
frontend
desktop-acceptance-contract
backend-linux
backend-windows
windows-msi-query
backend-macos
```

`scripts/ci/required-gate.mjs` is the single evaluator used by the workflow and
its unit tests. It accepts a JSON object with exactly those seven keys and accepts
only `result: success` for every key. Missing, extra, malformed, `failure`,
`cancelled`, `skipped`, and unknown results fail the gate. The stable displayed
check name is `CI / Required`; dependency job display names may evolve without
changing that external context.

### Toolchain facts

```text
Node       <- .node-version
pnpm       <- package.json#packageManager
Rust       <- rust-toolchain.toml [toolchain].channel and components
Python     <- .python-version
uv         <- the unique [[tools.uv]] entry in mise.lock
```

`scripts/ci/verify-toolchain.mjs` parses those sources without project
dependencies, resolves the locked uv and Python versions into GitHub step
outputs, and compares the installed runtimes with the same values. It must not
contain a second literal version set.

The `contracts` and `frontend` jobs resolve the uv version from `mise.lock`,
pass that output to the pinned `astral-sh/setup-uv` Action, disable its cache,
run `uv sync --locked`, and verify uv plus the managed Python before tests. The
frontend job must create this locked `.venv` before the full unit suite because
the development-hook behavior tests invoke the real Python harness.

## 3. Contracts

### Events, dependencies, and fail-closed aggregation

- Every trigger reaches the same seven unconditional dependency jobs and the
  same `CI / Required` aggregate job. A top-level path filter, conditional
  omission of a dependency job, or event-specific weaker gate is prohibited.
- `CI / Required` uses `if: always()` and receives all dependency results via
  `toJSON(needs)`. It must run even when a dependency fails, is cancelled, or
  is unexpectedly skipped.
- The evaluator owns both the exact dependency set and accepted result set.
  YAML-only string comparisons, success-by-default behavior, and accepting an
  unknown GitHub result are prohibited.
- Concurrency may cancel an obsolete run, but the latest candidate must still
  produce its own completed Required result. A cancelled candidate is not a
  passing source for merge or Release eligibility.

### Runners and toolchains

- Required CI uses only `ubuntu-24.04`, `windows-2022`, `windows-11-arm`, and
  `macos-15`; no `*-latest` runner is allowed.
- `windows-msi-query` is one unconditional required job with a two-entry
  native matrix: `windows-2022`/`X64` and
  `windows-11-arm`/`Arm64`, with `fail-fast: false`. It checks out the
  repository read-only, resolves Node/uv/Python only from the repository facts,
  performs `uv sync --locked`, verifies the managed Python, and executes the
  Trellis task-list protocol through `uv run --locked --no-sync`. Python's
  `sysconfig.get_platform()` must be `win-amd64` on the x64 leg and `win-arm64`
  on the ARM64 leg, so an emulated interpreter cannot satisfy the native
  environment gate. The same job then runs
  `tests/windowsInstallerQuery.integration.ps1`; it does not install
  pnpm, Rust, frontend, application, packaging, or signing dependencies. This
  is the required native evidence path for the Windows ARM64 environment
  contract without making mise an Actions dependency. The implementation and
  static contract are complete, but the Child 3 gate remains open until the
  closeout PR's x64/ARM64 matrix succeeds. The query fixture creates a temporary
  MSI from checked-in `.idt` source and exercises the production module on the
  runner's actual Windows Installer Automation boundary. A job-level
  `timeout-minutes: 15` bounds setup, Python/Trellis, COM, and fixture hangs
  instead of inheriting GitHub's six-hour default.
- `windows-11-arm` is a native hosted runner. Scheduling or image
  unavailability is a retryable infrastructure failure, but it still fails
  `CI / Required`; x64 substitution, cross-build, local Windows bridging, job
  omission, or a skipped ARM64 conclusion cannot satisfy this gate.
- CI never installs or invokes mise. GitHub Actions consume the repository's
  standard version files directly, and the verification helper compares the
  active runtime with those files.
- That prohibition includes indirect test discovery. The contracts job uses
  `release-check.mjs --ci`, which runs the pure Node lock, generated-doc, CI,
  Release, and local-build contracts without calling the local task runtime.
  The frontend Vitest run excludes only the five host-integration suites that
  deliberately execute real mise commands: development environment, Codex
  hooks, task API, generated task docs, and host system checks. The canonical
  local `mise run check` remains the blocking owner of their real-task-runtime
  cases. A dedicated CI step reruns the development-hook suite while excluding
  only its two real-mise wiring cases, so the locked Python harness, integrity,
  degradation, containment, and protocol behavior still fail the remote gate.
- `actions/setup-node` uses `node-version-file: .node-version`.
  `pnpm/action-setup` omits `version` and reads `packageManager`; it does not
  install dependencies implicitly.
- On Windows, the toolchain verifier launches the `pnpm.cmd` batch shim through
  `ComSpec` (falling back to `cmd.exe`) with explicit `/d /s /c` arguments and
  `shell: false`. Only its strict internal token character allowlist may enter
  the command string; whitespace, quoting, expansion, control, and shell
  metacharacters fail closed. Other tools and non-Windows invocation remain
  direct process launches.
- `actions-rust-lang/setup-rust-toolchain` omits toolchain, component, and
  target inputs so `rust-toolchain.toml` remains authoritative. Its own cache
  is disabled and `rustflags` is empty so it cannot override repository Cargo
  configuration.
- Rust check, Clippy, and tests use `--locked` on Linux, Windows, and macOS.
  Linux alone owns rustfmt and native Linux prerequisite installation.
  Windows uses `FYAGENT_WINDOWS_MANIFEST=test`; CI does not create a formal
  elevated Windows candidate.
- Full frontend tests run with deprecations promoted to failures. Focused
  pending-deprecation and Native Fetch behavior probes are owned by the
  dependency-removal contract and must remain in this gate once introduced.

### Immutable Actions and token permissions

- Every third-party Action reference is one reviewed 40-character commit SHA
  with a nearby version comment. Mutable major, tag, branch, or `latest`
  references are prohibited.
- The workflow default is exactly `contents: read`. CI does not request write
  permissions or access repository secrets.
- Checkout does not persist credentials. No dependency job may publish,
  attest, tag, modify a Release, or mutate repository configuration.

The pins reviewed for this implementation are:

| Action                                   | Version note | Commit                                     |
| ---------------------------------------- | ------------ | ------------------------------------------ |
| `actions/checkout`                       | `v7.0.1`     | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| `actions/setup-node`                     | `v7.0.0`     | `820762786026740c76f36085b0efc47a31fe5020` |
| `pnpm/action-setup`                      | `v6.0.10`    | `0977fd99725f1db4007ccb2928dbb4e90d06cc86` |
| `actions-rust-lang/setup-rust-toolchain` | `v1.17.0`    | `166cdcfd11aee3cb47222f9ddb555ce30ddb9659` |
| `astral-sh/setup-uv`                     | `v9.0.0`     | `c771a70e6277c0a99b617c7a806ffedaca235ff9` |
| `actions/labeler`                        | `v7.0.0`     | `bf12e9b00b37c5c0ca2b87b79b2daf7891dbda13` |

Changing a pin requires resolving the new official tag to its direct or peeled
commit again and rerunning the static workflow contract.

### Trusted-base Labeler

- The automatic event is `pull_request_target`, so the workflow definition and
  token authority come from the base repository rather than an untrusted fork.
- The Labeler workflow never checks out a branch, executes a shell step, runs
  pull-request code, restores a cache, uploads an artifact, or reads a secret.
  Without checkout, Labeler v7 fetches `.github/labeler.yml` through the API at
  `github.context.sha`, which is the base SHA for `pull_request_target`.
- Its only write permission is `pull-requests: write`; `contents: read` is
  needed to fetch the base configuration. `issues: write` is prohibited.
  Therefore every configured label must exist before the workflow runs; label
  creation is an explicit repository-maintenance action outside this workflow.
- Manual replay accepts one required numeric `pr_number` and uses the same
  pinned Action and configuration. It is not a path for executing PR code.

### Remote enforcement and accepted residual risk

The repository is public, but the approved v0.3.0 boundary deliberately does
not configure branch protection, a main/tag ruleset, or a protected Release
environment. The workflow can produce `CI / Required`, but GitHub does not
administratively require that context before a maintainer merge, and a change
with workflow-write authority could weaken a future workflow. This is an
accepted workflow-only source-protection risk, not evidence of protected main
or tags.

The current personal-account repository plus the no-protection/no-ruleset
decision cannot enable a real merge queue. The `merge_group(checks_requested)`
trigger and static contract are implemented, but no genuine merge-group run can
occur. The project owner accepted D113/D114 on 2026-08-08; D114 records that
live event as N/A under the current governance, not as a successful run. Its
accepted substitute evidence is the YAML trigger, fail-closed contract/static
tests, and real PR/main/manual runs. Do not remove the trigger or describe a
manual run as merge-group evidence.

The first pull request that introduces or changes a `pull_request_target`
workflow runs the Labeler version already present on the base branch. Automatic
Labeler evidence for the new version therefore begins only after the workflow
has reached the base branch; local tests and a manual run do not prove the
automatic event.

### Run observation discipline

Trigger authorization and run observation are separate gates. After an
Actions run is explicitly triggered, the initiating primary flow remains the
single owner of its evidence and synchronously waits for the whole run to reach
`completed`. It uses one blocking whole-run wait instead of delegating to a
background/asynchronous monitoring agent or issuing repeated `list`, `view`,
job-status, or check-status polling calls.

This is the active
[D117](../../../docs/fyagent/dev/v1-0.3.0/decisions/DECISION-REGISTER.md)
observation contract; it does not change trigger, retry, tag, or publication
authority.

Only after that wait returns does the primary flow read the final run and job
result once. A successful result needs no log sweep. A failed result permits
one retrieval of the failed-job logs for diagnosis; it does not authorize a
rerun, cancellation, workflow change, tag, or publication. This keeps the run,
job conclusion, and any failure log in one ordered evidence chain.

## 4. Validation & Error Matrix

| Condition                                                                                                         | Required result                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Any required dependency is missing, extra, malformed, failed, cancelled, skipped, or unknown                      | `CI / Required` fails and prints a machine-readable summary.                                                                      |
| Either native Windows contract matrix leg cannot schedule, prepare locked Python/Trellis, or pass its MSI fixture | `windows-msi-query` fails; Required remains red with no architecture fallback or conditional omission.                            |
| A dependency job is conditional or a workflow-level path filter hides the workflow                                | Static contract fails; do not merge the workflow change.                                                                          |
| A Required runner uses `*-latest` or an unapproved label                                                          | Static contract fails before remote execution.                                                                                    |
| A third-party Action is not a reviewed full SHA with a version note                                               | Static contract fails.                                                                                                            |
| Node, pnpm, Rust, uv, or Python differs from its repository fact                                                  | The affected job fails before its tests.                                                                                          |
| Windows batch invocation contains a token outside the internal allowlist                                          | Toolchain verification fails before `cmd.exe`; it never falls back to an implicit shell.                                          |
| `mise.lock` has no unique uv entry or setup-uv receives an independent literal                                    | Toolchain resolution fails; do not fall back to `latest`.                                                                         |
| Full unit tests run without `uv sync --locked` and the managed `.venv`                                            | Frontend CI fails; do not weaken the real hook harness.                                                                           |
| Labeler checks out or executes PR code, reads secrets, or gains another write permission                          | Reject the workflow as unsafe.                                                                                                    |
| A configured label does not exist                                                                                 | Labeler fails without `issues: write`; create the reviewed label out of band, then rerun.                                         |
| No real merge-group event can be produced under current repository governance                                     | Record the accepted D114 live-run N/A exception; require YAML/static plus real PR/main/manual evidence, and never report success. |
| A triggered run is delegated to an asynchronous monitor or sampled repeatedly                                     | Stop the observation path; the initiating flow must perform one synchronous whole-run wait.                                       |
| A completed successful run is followed by broad log retrieval                                                     | Reject the extra collection; fetch failed-job logs only after a failed final result.                                              |

## 5. Good / Base / Bad Cases

- Good: A pull request starts all seven jobs, every result is `success`, and the
  pure-Node evaluator completes the stable `CI / Required` context.
- Good: A fork pull request triggers Labeler from the base workflow, fetches
  the base configuration without checkout, and applies only existing labels
  with the pull-request token scope.
- Base: A manual CI dispatch runs the same complete gate. A manual Labeler
  replay labels one numeric PR with the same safe workflow.
- Bad: A `paths-ignore` rule prevents the Required context from appearing, a
  `skipped` result passes, setup-uv silently chooses latest, or a platform job
  hardcodes a second version.
- Bad: Labeler checks out `pull_request.head.sha`, executes a script, requests
  `issues: write`, or treats the workflow's privileged base token as safe for
  arbitrary PR content.

## 6. Tests Required

Run the canonical read-only contract and type gates:

```bash
mise run release:check
mise run typecheck
mise run format:check
```

`tests/githubWorkflowTriggers.test.ts`, `tests/ciWorkflow.test.ts`,
`tests/requiredCiGate.test.ts`, `tests/windowsInstallerQueryContract.test.ts`,
and `tests/ciToolchainContract.test.ts` enforce
the event, runner, Action, permission, dependency-result, and toolchain-source
contracts. They also ensure the CI commands cannot rediscover the five
mise-dependent host suites on a fresh runner. Run Prettier against the changed
workflows, helpers, tests, and this spec. Run actionlint when it is installed;
if it is unavailable, report that gap instead of claiming actionlint
validation.

Remote acceptance remains separate. Record the exact pull-request, main, and
manual run URLs and source SHAs. Under D114, the unavailable live merge-group
class is N/A only while the approved governance remains unchanged; local
trigger/static evidence plus those real runs form the accepted substitute, but
none may be described as a live merge-group success. Likewise, CI does not
prove native Release installers, asset attestations, publication, or branch
protection. For each authorized run, record the single completed run/job result
after the synchronous wait and attach failed-job logs only when that result
failed.

## 7. Wrong vs Correct

### Wrong

```yaml
on:
  pull_request:
    paths: ["src/**"]

jobs:
  required:
    if: success()
```

This can hide or skip the only stable context and cannot explain cancelled or
missing dependencies.

### Correct

```yaml
jobs:
  required:
    name: CI / Required
    if: always()
    needs:
      - contracts
      - frontend
      - desktop-acceptance-contract
      - backend-linux
      - backend-windows
      - windows-msi-query
      - backend-macos
```

Pass `toJSON(needs)` to the reviewed pure-Node evaluator and accept only the
exact all-success dependency set.
