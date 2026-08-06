# Sync upstream v3.19.1

## Goal

Integrate the exact upstream `farion1231/cc-switch` 3.19.1 release into the current
`feature/fyagent-v1` branch while preserving intentional FyAgent/project-specific
behavior and keeping the resulting history, code, and validation evidence auditable.

## Confirmed Facts

- Planning began on `feature/fyagent-v1` at
  `40e194b9bd24f91f57dd9fbe5fbb2de43527f99e`. During planning, independent
  concurrent work added `827bbba174fe0d171f05306258f745ff135ce627`
  (`chore(trellis): update templates to v0.6.12`) and
  `35abc8846d1c44e182d2c1edf0f5c39ef142714b`
  (`fix(trellis): use python3 in remaining command examples`). The latter is the
  current HEAD; both commits must be preserved.
- The working tree was clean at planning start.
- `origin` is `https://github.com/NongHua123/cc-switch.git`.
- `upstream` is `https://github.com/farion1231/cc-switch.git` for fetch and is
  disabled for push.
- No local `3.19.1` tag/ref was present at planning start.
- Remote verification establishes `v3.19.1` as an annotated tag object
  `7da48a05f51aa8099bc9dbdfae20d45a33fdc39a`, peeled to release commit
  `28529620f438b2ed25c812f6364825d846a4a9d6`.
- The release commit and current HEAD diverge at merge base
  `934a2d034859bcfb34df31e5cd46d6b35a28a80f`: the current project side has 56
  unique commits and the upstream side has 39 unique commits.
- A temporary, repository-external merge-tree analysis predicts 30 conflict
  paths: 28 content conflicts and two modify/delete conflicts. The concurrent
  Trellis update changed none of those paths or conflict types. The analysis did
  not alter this repository's refs, index, or worktree.
- The user prefers project-specific behavior when resolving conflicts, with each
  choice informed by the local modification history and the reason for the change.
- Local history and archived task contracts establish the following intentional
  project boundaries: the FyAgent clean-break identity, the independent `0.1.0`
  application version, the Codex Desktop installer and its narrow security
  boundary, WorkBuddy's isolated configuration domain, complete removal of the
  host upstream updater and commercial promotion surfaces, the mise-owned local
  toolchain, and project-specific CI/release workflows.
- Decisions that remain genuinely ambiguous after repository and history research
  must be returned to the user one at a time with a recommendation.
- Product-code changes and integration commands remain blocked until the final
  planning summary is explicitly approved.

## Requirements

- Identify and verify the exact upstream commit corresponding to release 3.19.1.
- Determine the current branch's fork relationship with upstream and inventory the
  upstream and local-only change sets before integration.
- Preserve intentional FyAgent branding, product behavior, compatibility work, and
  project-specific build/release behavior unless a reviewed decision explicitly
  supersedes it.
- Resolve conflicts semantically rather than by applying a blanket `ours` or
  `theirs` strategy; use code history, adjacent tests, and upstream intent as
  evidence for every non-trivial choice.
- Avoid unrelated refactors, new dependencies, or scope expansion.
- Keep the operation recoverable and record a precise pre-integration rollback
  point before changing the branch.
- Validate the integrated result in proportion to all affected layers, including
  the repository's required checks and focused checks for conflict-resolved areas.
- Retain and adapt upstream 3.19.1 security fixes for Skill ZIP extraction, SQL
  import isolation, terminal working-directory escaping, prototype-safe config
  traversal, deep-link handling, and related tests; local branding must not be
  used as a reason to drop those protections.
- Absorb compatible upstream functional changes such as native Responses provider
  support, corrected provider endpoints, query consolidation, icon-only app
  switching, translation completeness, pricing/session fixes, and proven dead-code
  deletion while reapplying the local WorkBuddy, accessibility, identity, and
  stability contracts.
- Audit cleanly merged additions as well as textual conflicts for stale
  `CC_SWITCH_TEST_HOME`, `cc_switch_lib`, former product identity, upstream updater
  infrastructure, and commercial promotion assets or metadata.

## Acceptance Criteria

- [ ] The current branch contains all intended upstream changes through the exact
      3.19.1 release commit.
- [ ] Intentional project-specific behavior remains present, and every substantive
      conflict decision has an evidence-backed rationale.
- [ ] The worktree contains no unresolved merge conflicts and no unrelated changes.
- [ ] Applicable formatting, lint, type-check, unit/integration, frontend, backend,
      renderer-build, and supported local desktop-build checks pass, or each
      unrun/failing check is reported with a concrete reason and residual risk.
- [ ] The final diff and commit graph match the user-approved integration strategy.
- [ ] The pre-integration rollback point and post-integration verification evidence
      are reported.
- [ ] A normal non-force push updates `origin/feature/fyagent-v1` only after local
      validation passes; no workflow, tag, pull request, or release is created.

## Out of Scope

- Synchronizing upstream changes newer than release 3.19.1.
- Opportunistic refactors or cleanup unrelated to the synchronization.
- Triggering remote CI or release workflows, pushing tags, publishing artifacts or
  releases, opening a pull request, or modifying repository workflow policy.
- Claiming Windows/macOS native installation, signing, notarization, packaging, or
  runtime acceptance from the WSL-only local evidence.

## Key Decisions

- Integrate the peeled `v3.19.1` release commit with a non-fast-forward merge
  commit on `feature/fyagent-v1`. Do not rebase the published project history,
  squash the upstream history, or cherry-pick the release commit set.
- Resolve conflicts with a layered semantic policy:
  - Project-owned product contracts are hard boundaries: FyAgent's clean-break
    identity and independent `0.1.0` version, no former-state compatibility,
    no host upstream updater or commercial promotion surfaces, WorkBuddy's
    isolated domain, the Codex Desktop installer security boundary, manual-only
    CI/release behavior, mise/toolchain declarations, cross-build boundaries,
    and Trellis infrastructure.
  - Upstream 3.19.1 security fixes and compatible functional improvements must
    be retained and adapted to those boundaries rather than discarded with a
    blanket `ours` choice.
  - Mixed files are merged field-by-field or symbol-by-symbol; blanket whole-file
    `ours`/`theirs` resolution is prohibited. A genuinely incompatible
    user-visible, compatibility, security, or release-cost decision returns to
    the user one item at a time.
- Validation is local-only. Do not trigger GitHub Actions CI, release workflows,
  or any other remote workflow. Windows/macOS conditional compilation, native
  installation, signing, notarization, and runtime acceptance remain explicitly
  unverified residual risks.
- After the local merge commit and all authorized local validation pass, push
  `feature/fyagent-v1` to `origin/feature/fyagent-v1` with a normal non-force
  push. This push includes the already committed concurrent Trellis v0.6.12
  update and the synchronization result, but must not dispatch any workflow,
  create or push a tag, or create a release.

## Risks and Deferred Validation

- The upstream annotated tag has no signature block. Exact tag-object and peeled
  commit hashes are verified against the requested upstream remote, but there is no
  cryptographic tag-signature evidence.
- The local host is WSL2 Ubuntu x86_64. It can exercise Linux compilation and the
  repository's cross-platform test doubles, but it cannot prove Windows/macOS
  conditional compilation, native install/launch, shell integration, signing,
  notarization, or Gatekeeper behavior.
- Remote CI is deliberately excluded, so Windows/macOS compile risk remains even
  after all local checks pass.
- `.github/FUNDING.yml` contains a likely stale sponsor anchor and one APINebula
  preset URL has ambiguous promotional provenance. Neither is changed unless the
  actual merge forces a decision; unrelated cleanup is deferred.
