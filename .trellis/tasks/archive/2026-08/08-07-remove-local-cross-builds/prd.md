# Remove local cross-platform builds — PRD

## Goal

Retire every local Linux/WSL-to-Windows/macOS cross-build capability without reducing native development or GitHub Actions release targets.

## Scope

- delete `scripts/windows-cross/**` and `scripts/macos-cross/**`
- first migrate Windows manifest, installer-actions, MSI structure, and signing-layer assertions into native Release workflow contracts or independent tests
- remove cross tasks, targets and `llvm-tools`
- remove dedicated tests and active spec
- update current docs and generated task catalog
- retain history with archive notices

## Constraints

- native host build remains
- standard `dev`/`build` tasks accept only the current host and expose no other OS/architecture target selector
- macOS Universal in macOS Actions remains
- ten formal release assets remain
- do not delete product-runtime optional mise compatibility

## Acceptance Criteria

- [x] no live cross scripts/tasks/targets/docs
- [x] no automatic `mise trust --yes`
- [x] native Windows/macOS/Linux builds and Actions design remain
- [x] contract scan prevents reintroduction
- [x] native Release contracts retain all Windows installer security assertions previously coupled to the deleted script

## Evidence Boundary

Implementation was authorized on 2026-08-08. The implementation and local/static
evidence are recorded in `research/removal-evidence.md`. Native Windows Installer
COM execution remains a formal Release-workflow gate and is not represented as a
Linux-host result.
