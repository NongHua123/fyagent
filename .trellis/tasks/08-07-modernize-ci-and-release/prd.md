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

- [ ] Required gate handles failure/cancel/skip correctly
- [ ] all supported platform jobs prove pinned versions
- [ ] unsigned full matrix passes
- [ ] automatic Labeler never checks out or executes PR code and has only `pull-requests: write`
- [ ] formal eligibility proves `v0.3.0`, product version, `origin/main` ancestry, same-SHA Required success, and repository/workflow identity
- [ ] exact 10 assets, digests, build metadata, and attestations pass; no capability-gap downgrade is allowed for v0.3.0
- [ ] final publish is stable/non-prerelease and only obtains `contents: write` after every gate succeeds
- [x] standard local entrypoints reject or omit non-host targets and the current
      Linux x64 environment retains only `x86_64-unknown-linux-gnu`
- [x] local Windows diagnostic processes/output were cleaned and prior local
      Light/MSI results are classified as diagnostic-only, never acceptance
- [ ] each authorized Actions run is observed by the initiating main flow with
      one synchronous wait through `completed`, one final run/job result read,
      and failed-job logs only after failure

## Evidence Boundary

This task begins in `planning`. Its implementation can be committed before remote runs, but it remains open until the implementation PR, main CI, unsigned preflight, formal Release, and independent post-publication verification produce real evidence.

Local cleanup and contract checks cannot satisfy a remote gate. PR/main/manual
CI, preflight, formal Release, and post-publication evidence remain pending and
keep the task and parent at NO-GO until they occur.
