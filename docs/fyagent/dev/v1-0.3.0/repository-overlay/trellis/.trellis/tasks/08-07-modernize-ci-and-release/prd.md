# Modernize CI and Release workflows — PRD

## Goal

Make GitHub Actions the strict multi-platform merge and formal-release authority with pinned runners/actions, minimum permissions, reproducible toolchains, and a fail-closed ten-asset transaction.

## Scope

- automatic PR/main/merge-group CI
- stable Required aggregate gate
- explicit runners and runtime tool checks
- same-arch Ubuntu 22.04 release containers
- full-SHA Action pins and permissions
- unsigned/signed preflight modes
- signing/notarization, asset manifest and provenance

## Constraints

- specific job topology may adapt to merged repository
- no mise in Actions
- no `*-latest` in required/release jobs
- no publish write before all builds pass
- no QEMU/cross-architecture Linux release
- no local formal release

## Acceptance Criteria

- [ ] Required gate handles failure/cancel/skip correctly
- [ ] all supported platform jobs prove pinned versions
- [ ] unsigned full matrix passes
- [ ] signed protected preflight passes before production
- [ ] exact 10 assets, digests and attestations/recorded capability gap

## Evidence Boundary

This task begins in `planning`. Nothing in this artifact claims the merge, configuration, tests, CI, or Release has already been completed. Pending platform/Git evidence must be attached during implementation.
