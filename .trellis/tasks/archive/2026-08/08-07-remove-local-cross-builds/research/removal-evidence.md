# Local cross-build removal evidence

## Result

Commit `e8954d97faed1b833a6bce6fb9477b4fc4e2fd83` removes every
repository-owned Linux/WSL-to-macOS or Windows build path while preserving
current-host development and the five native GitHub Actions release target
groups.

The Windows MSI assertions that previously depended on the deleted Linux
cross-build script now run in the native Windows Release matrix through
`scripts/release/verify-windows-msi.ps1`. The verifier opens the candidate MSI
read-only, validates its product/protocol/payload/architecture contracts,
extracts `Binary.FyAgentInstallerActions`, and binds that stream to the helper
built in the same job by PE machine, byte length, and SHA-256.

## Removed surfaces

- `scripts/macos-cross/**`
- `scripts/windows-cross/**`
- the five local cross-build mise tasks
- non-host Rust targets and `llvm-tools`
- `tests/macosCrossWorkflow.test.ts`
- `.trellis/spec/backend/wsl-macos-cross-build.md`
- active documentation entry points for the retired paths

Historical design inputs and archived tasks remain unchanged as audit history.

## Independent check

The independent staged-diff review added two contract hardening corrections:

1. MSI host-residue scanning now includes the `Feature`,
   `FeatureComponents`, `Upgrade`, `Dialog`, `Control`, and
   `MsiLockPermissionsEx` tables.
2. The mise trust regression matcher was narrowed to actual `mise`, absolute
   mise-path, or `$MISE*` invocations, avoiding false positives on unrelated
   uses of the word `trust` while still rejecting `trust` and `untrust`.

The targeted gates were rerun after these corrections.

## Verification

The implementation pass completed the following checks successfully:

- `mise lock --platform linux-x64,linux-arm64,macos-x64,macos-arm64,windows-x64,windows-arm64`
- `mise exec -- pnpm test:unit` — 129 Vitest files passed
- `mise exec -- pnpm typecheck`
- `mise exec -- pnpm format:check`
- `mise exec -- cargo fmt --all --check --manifest-path src-tauri/Cargo.toml`
- `mise exec -- cargo clippy --workspace --all-targets --locked --manifest-path src-tauri/Cargo.toml -- -D warnings`
- `mise exec -- cargo test --workspace --locked --manifest-path src-tauri/Cargo.toml` — 2,627 main-library tests plus integration and installer-actions suites passed
- `mise exec -- pnpm run version:check` — the intentional pre-Child-3 version remained `0.2.1`

After independent review corrections, the final staged content passed:

- `mise exec -- pnpm vitest run tests/localBuildBoundary.test.ts tests/releaseWorkflow.test.ts` — 21/21 tests
- `mise exec -- pnpm typecheck`
- `mise exec -- pnpm format:check`
- `mise exec -- pnpm run version:check`
- `python3 .trellis/scripts/task.py validate .trellis/tasks/08-07-remove-local-cross-builds`
- `mise tasks validate --errors-only`
- `git diff --cached --check`
- exact negative scans for removed paths/tasks/specs, Rust cross targets,
  `llvm-tools`, and repository-controlled mise trust mutation

## Evidence boundary

The Linux implementation environment has neither PowerShell nor Windows
Installer COM. It therefore cannot execute the final PowerShell verifier or
claim native MSI lifecycle acceptance. Native Windows x64 and ARM64 workflow
runs must still execute the verifier against their actual bundles. UAC,
install/repair/upgrade/uninstall, ICE, embedded executable manifest, and the
eventual unsigned-state checks remain formal Release gates owned by the
CI/Release child and parent acceptance.
