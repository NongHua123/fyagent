# Modernize CI and Release workflows — PRD

## Goal

Make GitHub Actions the strict multi-platform merge and formal-release authority
with pinned runners/actions, minimum permissions, reproducible toolchains, and
a fail-closed ten-asset transaction. Local development remains strictly
host-native, and every triggered remote run produces one synchronous,
whole-run evidence chain.

## Scope

- automatic PR/main/merge-group CI
- stable Required aggregate gate
- explicit runners and runtime tool checks
- same-arch Ubuntu 22.04 release containers
- full-SHA Action pins and permissions
- safe automatic `pull_request_target` Labeler plus manual fallback
- unsigned full-matrix preflight and stable `v0.3.0` formal mode only
- exact installer allowlist, SHA-256 manifest, build metadata, and mandatory GitHub artifact attestations
- workflow-only source eligibility and explicit residual-risk documentation
- host-native-only local development, build, test, package, and verification
- synchronous whole-run Actions waiting with one final result read and
  failure-only failed-job log retrieval

## Constraints

- specific job topology may adapt to merged repository
- no mise in Actions
- no `*-latest` in required/release jobs
- no publish write before all builds pass
- no QEMU/cross-architecture Linux release
- no local formal release
- no local cross-OS or cross-architecture compilation, packaging, or
  verification through target flags, subsystem bridges, foreign executables,
  emulators, copied toolchains, or staged non-host artifacts
- no background/asynchronous Actions monitor and no repeated status polling
- no signed mode, Windows/macOS signing, notarization, staple, signing secrets, or Release environment
- no main/tag ruleset or branch protection configuration, and no claim that administrator protections exist

## Acceptance Criteria

- [x] Required gate handles failure/cancel/skip correctly
- [x] all release platform jobs prove pinned versions
- [x] unsigned full matrix passes
- [x] automatic Labeler never checks out or executes PR code and has only `pull-requests: write`
- [x] formal eligibility proves `v0.3.0`, product version, `origin/main` ancestry, same-SHA Required success, and repository/workflow identity
- [x] exact 10 assets, digests, build metadata, and attestations pass; no capability-gap downgrade is allowed for v0.3.0
- [x] final publish is stable/non-prerelease and only obtains `contents: write` after every gate succeeds
- [x] standard local entrypoints reject or omit non-host targets and the current
      Linux x64 environment retains only `x86_64-unknown-linux-gnu`
- [x] local Windows diagnostic processes/output were cleaned and prior local
      Light/MSI results are classified as diagnostic-only, never acceptance
- [x] each authorized implementation/release Actions run is observed by the initiating main flow with
      one synchronous wait through `completed`, one final run/job result read,
      and failed-job logs only after failure
- [x] the closeout PR's expanded Windows x64/ARM64 locked uv/Python/Trellis
      smoke passes natively before task archival

## Evidence Boundary

PR #7, exact-main Required CI, unsigned preflight, formal Release, and
independent post-publication verification now provide real evidence and make
the release gate GO. PR #8 run `31264604075` at head
`623b6924e3b8682321b26aa69c15dc6f0b9f6f09` failed closed: x64 job
`93120609402` passed, ARM64 job `93120609411` failed because setup-uv's
version-only request selected `win-amd64`, and Required job `93121912798`
failed. Commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` switched to a full
uv request with managed Python. Run `31265504901` then passed x64 job
`93122857985`, ARM64 job `93122858012`, and Required job `93123992476`, so
the expanded native gate is complete. The final design-package manifest is
rebuilt and verified. D114 remains N/A. Ordered Child 3 → Child 4 → Child 6 →
parent archive, journal, final PR CI/merge, exact-main CI, and branch cleanup are
not claimed by this record.
