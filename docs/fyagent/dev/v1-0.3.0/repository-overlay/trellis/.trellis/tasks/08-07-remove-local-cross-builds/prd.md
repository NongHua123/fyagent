# Remove local cross-platform builds — PRD

## Goal

Retire every local Linux/WSL-to-Windows/macOS cross-build capability without reducing native development or GitHub Actions release targets.

## Scope

- delete `scripts/windows-cross/**` and `scripts/macos-cross/**`
- remove cross tasks, targets and `llvm-tools`
- remove dedicated tests and active spec
- update current docs and generated task catalog
- retain history with archive notices

## Constraints

- native host build remains
- macOS Universal in macOS Actions remains
- ten formal release assets remain
- do not delete product-runtime optional mise compatibility

## Acceptance Criteria

- [ ] no live cross scripts/tasks/targets/docs
- [ ] no automatic `mise trust --yes`
- [ ] native Windows/macOS/Linux builds and Actions design remain
- [ ] contract scan prevents reintroduction

## Evidence Boundary

This task begins in `planning`. Nothing in this artifact claims the merge, configuration, tests, CI, or Release has already been completed. Pending platform/Git evidence must be attached during implementation.
