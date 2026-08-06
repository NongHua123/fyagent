# Upstream v3.19.1 Synchronization Design

## 1. Context and Integration Boundary

The synchronization preserves two real histories rather than translating one
side into the other:

- approved local base at planning completion:
  `35abc8846d1c44e182d2c1edf0f5c39ef142714b`
- requested upstream annotated tag: `v3.19.1`
- tag object: `7da48a05f51aa8099bc9dbdfae20d45a33fdc39a`
- peeled release commit: `28529620f438b2ed25c812f6364825d846a4a9d6`
- merge base: `934a2d034859bcfb34df31e5cd46d6b35a28a80f`
- local/upstream unique commits at planning completion: `56 / 39`
- predicted textual conflicts: 30 paths (`28` content, `2` modify/delete)

The final integration is a non-fast-forward merge commit whose first parent is
the current `feature/fyagent-v1` HEAD and whose second parent is the peeled
`v3.19.1` commit. Rebase, squash, cherry-pick, force-push, and tag rewriting are
outside the design.

If the current branch changes again before execution, preflight must recompute
the merge base, left/right counts, and `merge-tree` conflict set. A product-code
change or changed conflict set invalidates this design and returns to planning;
a non-overlapping Trellis-only commit may be preserved after recording the new
rollback hash and proving that the conflict set is unchanged.

## 2. Product and Engineering Invariants

### Project-owned hard boundaries

The merge result must preserve:

- the FyAgent clean-break identity (`FyAgent`, `fyagent`,
  `com.fyagent.desktop`, `fyagent://`, `~/.fyagent`, `fyagent.db`,
  `FYAGENT_*`, and other owned serialization/storage names);
- no discovery, migration, aliasing, import, or cleanup of former CC Switch
  state;
- the independent application version `0.1.0` in npm, Cargo, and Tauri;
- the absence of the host upstream updater, updater artifacts, upstream update
  key/endpoints, and CC Switch R2/updater synchronization;
- the narrow Codex Desktop installer and trusted restart security boundaries;
- WorkBuddy as an isolated top-level configuration domain;
- the latest Codex native-capability behavior from `dd4162e0`;
- sponsor-free/partner-free presentation and non-tracking Provider URLs;
- the manual-only CI and tag-gated release policy;
- the repository-owned mise toolchain, Windows/macOS cross-build boundaries,
  approved FyAgent brand assets, and Trellis/Codex infrastructure;
- factual repository provenance, history, licenses, authorship, and upstream
  attribution where former-name strings are historical or external facts.

### Upstream semantics that must be retained

The local contracts are not a blanket reason to reject upstream work. The merge
must adapt and keep, when applicable:

- Skill ZIP traversal, symlink, archive-budget, credential, and panic hardening;
- SQL import protection against cross-file operations;
- terminal working-directory escaping and prototype-safe configuration walking;
- deep-link risk handling, URL-safe Base64, and safe usage-script defaults;
- DeepSeek, Volcengine Agentplan, and Tencent Hunyuan native Responses support;
- valid Provider endpoint, catalog, model, pricing, usage, session, Grok Build,
  protocol-conversion, query, and translation fixes;
- upstream dead-code/dependency deletion where the deleted code has no remaining
  FyAgent consumer.

Mixed files are resolved field-by-field or symbol-by-symbol. Whole-file
`ours`/`theirs` resolution is not an accepted shortcut.

## 3. Merge Transaction

After final approval and Trellis task activation:

1. Record the exact pre-merge HEAD as the rollback point and verify the working
   tree contains only the active task artifacts as expected.
2. Fetch only the requested `v3.19.1` tag into the local repository.
3. Verify the fetched tag object and peeled commit against the hashes above.
4. Re-run topology and merge-tree checks against the then-current HEAD.
5. Start `git merge --no-ff --no-commit v3.19.1`.
6. Resolve and validate the complete merge while `MERGE_HEAD` remains present.
7. Create `chore(sync): merge upstream v3.19.1` only after all authorized local
   gates pass.

Keeping the merge uncommitted during validation preserves `git merge --abort` as
the clean rollback path. The active task directory is not staged into the merge
commit; task archival remains a separate project-history commit.

## 4. Conflict Resolution Matrix

### 4.1 Identity, version, dependencies, and release (6 paths)

Paths:

- `.github/workflows/release.yml`
- `package.json`
- `pnpm-lock.yaml`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Resolution:

- retain FyAgent name/version/identifier/scheme/artifact paths and
  `createUpdaterArtifacts: false`;
- retain `tauri-plugin-store` and project-only installer/cross-build
  dependencies; do not restore `tauri-plugin-updater`;
- incorporate compatible upstream dependency removal after verifying no local
  consumers remain;
- add upstream's `Win32_Storage_FileSystem` feature to the union of required
  Windows features;
- preserve the manual branch macOS artifact and tag-only release constraints;
- consider upstream checksum/timestamp hardening only where it fits the FyAgent
  release path without restoring an updater or CC Switch endpoint;
- exclude the clean-added `.github/workflows/sync-r2.yml` because it exists to
  serve the removed CC Switch updater/R2 channel and has no approved FyAgent
  distribution contract;
- resolve manifests first, then mechanically regenerate lockfiles from those
  reviewed manifests instead of hand-editing long conflict blocks.

### 4.2 README branding and sponsor content (4 paths)

Paths: `README.md`, `README_DE.md`, `README_JA.md`, `README_ZH.md`.

Retain the FyAgent title, `Why FyAgent?`, real `NongHua123/cc-switch`
provenance, and sponsor-free structure. Absorb only compatible upstream factual
or functional documentation. Do not restore Sponsor sections, campaign URLs,
affiliate parameters, or CC Switch as the current product name.

### 4.3 Modify/delete dead code (2 paths)

Paths: `scripts/extract-icons.js`, `src/lib/schemas/settings.ts`.

Accept the upstream deletion. Local-only changes were mechanical identity
renames and no runtime consumer remains. TypeScript/import searches must prove
that deletion does not leave a local reference.

### 4.4 Backend security behavior and tests (2 paths)

Paths: `src-tauri/src/database/backup.rs`,
`src-tauri/src/services/skill.rs`.

Retain both sides' intent:

- keep upstream SQL and archive hardening plus all associated regression tests;
- keep FyAgent export-header, identity, storage, and clean-break tests;
- adapt test isolation and expected values to `FYAGENT_TEST_HOME`,
  `FYAGENT_SQL_EXPORT_HEADER`, `~/.fyagent`, and `fyagent_lib` before running
  any affected test.

Running an unadapted upstream test is prohibited because `CC_SWITCH_TEST_HOME`
is ignored by current production code and may allow a test to inspect the real
user profile.

### 4.5 App, switcher, proxy queries, and integration test (4 paths)

Paths:

- `src/App.tsx`
- `src/components/AppSwitcher.tsx`
- `src/hooks/useProxyStatus.ts`
- `tests/integration/App.test.tsx`

Use the upstream icon-only switcher and consolidated query structure as the
structural base. Reapply WorkBuddy/configuration-domain `enabled` gating,
retain `aria-pressed`, combine it with upstream `title` and `aria-label`, and
keep the 15-second integration-test timeout. Remove all references to upstream-
deleted `useAutoCompact` and other dead query paths.

### 4.6 Provider presets (8 paths)

Paths are the Claude Desktop, Claude, Codex, Gemini, Grok Build, Hermes,
OpenClaw, and OpenCode preset modules under `src/config/`.

For every preset:

- retain upstream endpoint, model, `apiFormat`, native Responses, failover, and
  domain-migration fixes;
- remove `isPartner`, `primePartner`, `partnerPromotionKey`, campaign copy, and
  referral/affiliate query parameters;
- keep a valid A6API Provider as a neutral functional preset while removing its
  sponsor banner, partner metadata, referral data, and promotional locale keys;
- keep only assets required for a neutral Provider selector; exclude clean-added
  promotional banners.

This implements the approved rule that promotion mechanisms are removed while
functional third-party Providers remain available.

### 4.7 Four locales (4 paths)

Paths: all files under `src/i18n/locales/{en,ja,zh,zh-TW}.json`.

Retain new security, deep-link, models.dev, Grok, tool-management, and missing-key
translations. Remove sponsor/promotion namespaces and copy. Use FyAgent for
current-product text, but do not rewrite third-party codes, protocol fields,
historical records, or factual external identifiers. Parse every JSON file and
verify four-locale key parity through existing tests.

## 5. Clean-Merge Semantic Audit

Textual conflicts are not the full work set. Before tests, audit every upstream-
added/modified path for:

- `CC_SWITCH_TEST_HOME` and other former owned environment variables;
- `cc_switch_lib` imports in `src-tauri/tests/provider_service.rs` and elsewhere;
- former current-product identity in runtime/configuration;
- upstream updater keys, endpoints, permissions, plugins, UI, and workflows;
- `dl.ccswitch.io`, `farion1231`, or former-name values, classifying each as an
  invalid active dependency or a legitimate provenance/history/external fact;
- sponsor/partner/referral metadata and clean-added promotion graphics;
- references to modules deleted by upstream's dead-code cleanup;
- WorkBuddy or Codex native-capability regressions in cleanly merged code.

The existing identity, version, release-workflow, CI-trigger, cross-build, and
feature-contract tests are authoritative guards; broad string replacement is
not an audit method.

## 6. Lockfile Strategy

`package.json` and `src-tauri/Cargo.toml` are reviewed sources of truth.

- Seed `pnpm-lock.yaml` from the local side only to remove conflict markers,
  then run the mise-owned pnpm lockfile update with lifecycle scripts disabled.
- Seed `src-tauri/Cargo.lock` from the local side, then let Cargo reconcile the
  reviewed manifest once without `--locked`.
- Inspect both generated diffs for unexpected package upgrades, former root
  names, updater dependencies, or loss of local platform dependencies.
- All subsequent installs/checks use `--frozen-lockfile` or `--locked`.

Lockfile generation is a mechanical consequence of reviewed manifests, not an
opportunity to upgrade unrelated dependencies.

## 7. Validation Boundary

Authorized local validation includes:

- conflict/index and whitespace checks;
- focused tests for all conflict classes and security changes;
- identity, former-test-home, crate-name, updater, promotion, and unresolved-
  import audits;
- frontend type-check, formatting check, complete Vitest suite, and renderer
  production build;
- Rust format check, strict Clippy, and complete Rust tests with the reconciled
  lockfile;
- a no-bundle Tauri desktop build when current Linux/WSL native prerequisites
  permit it.

Remote CI, remote release workflows, tags, packaged-release publication, and
native Windows/macOS acceptance are excluded. A local command that cannot run
must be reported with the exact reason and residual risk; it is not silently
reclassified as passing.

## 8. Commit, Push, and Remote Concurrency

After validation:

1. Create the merge commit and verify it has exactly the expected two parents.
2. Verify `v3.19.1^{}` is an ancestor of the new HEAD and local history remains
   the first-parent line.
3. Finish and archive the Trellis task using the project workflow; keep task
   bookkeeping in its own commit if the workflow creates one.
4. Fetch `origin/feature/fyagent-v1` immediately before push and require it to
   remain an ancestor of local HEAD.
5. Push with a normal non-force branch refspec only. Do not use `--tags`,
   `--follow-tags`, `--force`, or `--force-with-lease`.
6. Verify the remote branch hash after push. Do not dispatch any workflow.

If origin advances incompatibly, stop and request direction; do not merge,
rebase, or overwrite the remote branch without a new decision.

## 9. Rollback and Stop Conditions

Before the merge commit, any unresolved product ambiguity, tag mismatch,
unexpected branch change, or irreducible failure uses `git merge --abort` and
returns to the recorded pre-merge HEAD. No reset is required.

After a local merge commit but before push, a failed graph/audit check blocks
the push and is corrected locally without rewriting pre-existing history.
After push, rollback must be a new `git revert -m 1 <merge-commit>` change;
published history is never reset or force-pushed.

Stop and ask the user when a conflict cannot simultaneously preserve an approved
project boundary and an upstream security/functional requirement, or when a
remote branch movement would require changing the approved topology.

