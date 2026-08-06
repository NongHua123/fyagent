# Upstream v3.19.1 Synchronization Execution Plan

## Completion Definition

The task is complete only when the exact upstream `v3.19.1` release commit is a
parent-line ancestor of a validated non-fast-forward merge commit on
`feature/fyagent-v1`, all approved FyAgent contracts and intended upstream fixes
are present, the authorized local quality gates have passed or are explicitly
accounted for, the task is archived, and a normal push has updated the matching
origin branch without dispatching remote CI or release automation.

## Phase A: Approval, Activation, and Preflight

- [ ] Receive explicit approval of the final planning summary in a new user
      message.
- [ ] Run `task.py start` for this task; do not edit product code while the task
      remains in `planning`.
- [ ] Record current branch, exact HEAD, `origin` tracking state, status, and
      active task path.
- [ ] If HEAD differs from the approved baseline, classify the new commits and
      repeat repository-external merge-tree analysis. Return to planning if a
      product path or conflict set changed.
- [ ] Require a clean tracked worktree. Preserve the active untracked task
      directory and every concurrent user commit.
- [ ] Record the exact pre-merge HEAD as the rollback point.
- [ ] Fetch only upstream tag `v3.19.1` and verify:
  - tag object `7da48a05f51aa8099bc9dbdfae20d45a33fdc39a`;
  - peeled commit `28529620f438b2ed25c812f6364825d846a4a9d6`;
  - merge base and left/right counts are understood;
  - neither side is already an ancestor of the other.
- [ ] Re-run `git merge-tree` or equivalent dry analysis and compare the actual
      conflict set with the approved 30-path matrix.

Rollback: no product mutation has occurred. A tag/hash mismatch or material
topology change stops the task before merge.

## Phase B: Start the Transaction

- [ ] Run `git merge --no-ff --no-commit v3.19.1`.
- [ ] Capture `git status`, unmerged index entries, and the actual conflict list.
- [ ] Confirm `MERGE_HEAD` is the peeled release commit.
- [ ] Keep the active Trellis task directory unstaged from the merge commit.

Rollback: `git merge --abort` restores the recorded pre-merge state.

## Phase C: Resolve the 30 Textual Conflicts

### C1. Manifests and release boundary

- [ ] Resolve `.github/workflows/release.yml` with FyAgent/manual/tag-gated
      behavior and compatible upstream hardening only.
- [ ] Resolve npm/Cargo/Tauri manifests with FyAgent `0.1.0`, local package/lib
      names, no updater, required installer/cross-platform dependencies, and
      upstream `Win32_Storage_FileSystem`.
- [ ] Remove clean-added `.github/workflows/sync-r2.yml` from the merge result.
- [ ] Leave lockfiles seeded from the local side until Phase E regeneration.

### C2. README and dead code

- [ ] Resolve the four READMEs as current FyAgent, sponsor-free documentation
      with factual repository/upstream provenance.
- [ ] Accept deletion of `scripts/extract-icons.js` and
      `src/lib/schemas/settings.ts`.

### C3. Security paths

- [ ] Combine SQL import hardening with FyAgent export/identity tests in
      `database/backup.rs`.
- [ ] Combine Skill archive hardening with FyAgent storage/isolation tests in
      `services/skill.rs`.
- [ ] Replace stale test-only former identity variables before executing tests.

### C4. App/query/accessibility

- [ ] Adopt icon-only AppSwitcher and consolidated query ownership.
- [ ] Reapply WorkBuddy and configuration-domain query gating.
- [ ] Combine `aria-pressed`, `title`, and `aria-label` behavior.
- [ ] Remove dead `useAutoCompact`/proxy-query imports.
- [ ] Retain the 15-second integration-test timeout.

### C5. Provider presets and locales

- [ ] Merge all eight preset modules field-by-field, keeping functional upstream
      endpoint/native-Responses/model/failover changes.
- [ ] Strip partner/promotion fields and referral/affiliate query parameters.
- [ ] Retain A6API only as a neutral functional Provider; remove its banners and
      promotional metadata/text.
- [ ] Resolve all four locale files with upstream functional/security keys,
      FyAgent current-product branding, and no promotion namespace.

Exit criterion: `git ls-files -u` and conflict-marker search both report no
remaining conflict, but semantic audit has not yet been declared complete.

## Phase D: Audit Cleanly Merged Changes

- [ ] Search changed runtime/config/test files for former owned identity and
      classify every hit; preserve only factual provenance/history/external
      identifiers and explicit negative tests.
- [ ] Replace upstream `CC_SWITCH_TEST_HOME` with the owned isolated test-home
      mechanism before any affected Rust test.
- [ ] Replace `cc_switch_lib` call sites with `fyagent_lib` where they refer to
      the current crate.
- [ ] Verify the host updater remains absent across Cargo/npm, Tauri config,
      capabilities, Rust commands/plugins, frontend, and workflows.
- [ ] Verify no sponsor/partner/referral metadata or promotional graphic entered
      the current product surface.
- [ ] Search for references to upstream-deleted modules and query paths.
- [ ] Confirm WorkBuddy isolation, Codex native capabilities, installer IPC,
      trusted restart, identity, release, CI-trigger, cross-build, and brand-asset
      contracts still map to their existing tests/specs.
- [ ] Review every automatically merged upstream security commit's affected path
      so clean merge is not mistaken for verified behavior.

Exit criterion: all semantic conflicts are either resolved under the approved
policy or surfaced to the user as a single blocking decision.

## Phase E: Reconcile Generated Dependency State

- [ ] Regenerate `pnpm-lock.yaml` from the reviewed `package.json` using the
      mise-owned pnpm with lockfile-only and lifecycle scripts disabled.
- [ ] Reconcile `src-tauri/Cargo.lock` once from the reviewed Cargo manifest
      without `--locked`.
- [ ] Inspect lockfile diffs for unrelated upgrades, former root package names,
      updater dependencies, or missing local platform dependencies.
- [ ] From this point forward, use `--frozen-lockfile` and `--locked`.

Rollback: manifest or lockfile drift that cannot be explained by the approved
merge blocks the commit; abort the merge or revise only the reviewed manifest.

## Phase F: Focused Validation

- [ ] Verify no unmerged entries, conflict markers, or whitespace errors.
- [ ] Parse changed JSON/TOML/YAML through repository tools/tests.
- [ ] Run the project synchronization guards:
  - `tests/versionConsistency.test.ts`;
  - `tests/releaseWorkflow.test.ts`;
  - `tests/githubWorkflowTriggers.test.ts`;
  - `tests/macosCrossWorkflow.test.ts`.
- [ ] Run focused frontend tests for App/AppSwitcher, Provider presets, Provider
      native capabilities, locale behavior, WorkBuddy, and query changes.
- [ ] Run focused Rust tests for database backup/import, Skill archive install,
      Provider service/Codex config, deeplink, terminal handling, pricing/session,
      and affected platform-independent installer behavior.
- [ ] Confirm focused tests use temporary homes and do not touch the real user
      profile.

Any focused failure is resolved while the merge remains uncommitted, followed by
the same focused suite again.

## Phase G: Full Authorized Local Quality Gate

Run through the repository-owned mise environment:

- [ ] `mise exec -- pnpm install --frozen-lockfile`
- [ ] `mise exec -- pnpm typecheck`
- [ ] `mise exec -- pnpm format:check`
- [ ] `mise exec -- pnpm test:unit`
- [ ] `mise exec -- pnpm run build:renderer`
- [ ] `mise exec -- cargo fmt --check --manifest-path src-tauri/Cargo.toml`
- [ ] `mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml --locked -- -D warnings`
- [ ] `mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] `mise exec -- pnpm tauri build --no-bundle` when current WSL/Linux native
      prerequisites support it; otherwise capture the exact blocker.
- [ ] `git diff --check`
- [ ] Compare status before/after validation and reject unexpected source or
      lockfile writes.

Do not install `actionlint`, add a dependency, trigger GitHub Actions, or run a
release workflow merely to widen validation. Windows/macOS native and signing
acceptance remain deferred.

## Phase H: Create and Verify the Merge Commit

- [ ] Review staged diff/stat and substantive conflict resolutions.
- [ ] Ensure active task artifacts are not staged in the merge commit.
- [ ] Commit as `chore(sync): merge upstream v3.19.1`.
- [ ] Verify the merge commit has exactly two expected parents.
- [ ] Verify `v3.19.1^{}` is an ancestor of HEAD and the pre-merge local commit
      remains first parent.
- [ ] Re-run final status, diff/check, identity/updater/promotion audits that do
      not depend on uncommitted state.

No push occurs if any graph or audit check fails.

## Phase I: Trellis Check, Finish, and Push

- [ ] Run the `trellis-check` workflow against the full synchronization scope;
      fix any validated issue and repeat affected local gates.
- [ ] Update project specs only if the merge intentionally changes a durable
      project contract; do not rewrite specs merely to document unchanged
      upstream internals.
- [ ] Archive/finish the Trellis task according to project workflow and commit
      task bookkeeping separately when required.
- [ ] Fetch the current origin branch and require
      `origin/feature/fyagent-v1` to be an ancestor of local HEAD.
- [ ] Push only `HEAD:refs/heads/feature/fyagent-v1` with a normal non-force
      push; do not push or follow tags.
- [ ] Verify the remote branch hash matches local HEAD.
- [ ] Do not dispatch CI, release, or any other workflow.

If origin has advanced incompatibly, stop and ask the user. Never force-push or
silently integrate new remote work outside this approved plan.

## Final Evidence Package

- exact upstream tag object, release commit, merge base, and merge parents;
- pre-merge rollback hash and final local/remote hashes;
- actual conflict list and per-category resolution rationale;
- diff/stat and audits proving FyAgent/updater/promotion boundaries;
- focused and full local commands with exit codes/summaries;
- explicit unverified Windows/macOS/remote-CI/native-release risks;
- confirmation that the push was branch-only and no workflow/tag/release was
  triggered.

