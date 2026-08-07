# FyAgent 0.3.0 Installer and Version Contract

## 1. Scope / Trigger

Read this contract before changing any FyAgent application version, release-tag
rule, release asset name, download manifest, Windows MSI template, installer
custom action, or native Windows MSI release gate.

The current implementation baseline is application version 0.3.0. The package
at docs/fyagent/dev/v1-0.3.0/ is this iteration's requirements, decisions, and
evidence workspace. Preserve the original input intent and Git history, but
revise the package auditably as implementation evidence becomes available.
Child 6 regenerates MANIFEST.sha256 only after the final evidence update; do not
claim the pre-closeout manifest proves bytes that are still being revised.

This is a cross-layer contract:

- Cargo metadata supplies the app version to Tauri and every local build.
- Node version commands validate or change only that canonical version and its
  local lockfile package blocks.
- The release workflow freezes version, tag, and source SHA before any platform
  build starts.
- The Windows MSI embeds an architecture-matched native validation DLL and
  applies the same directory policy to UI and silent installation.

It does not authorize a toolchain upgrade, a Git tag, remote workflow dispatch,
signing, notarization, or a public release.

## 2. Signatures

### Application version source

```toml
[workspace]
members = [".", "installer-actions"]
resolver = "2"

[workspace.package]
version = "X.Y.Z"

[package]
name = "fyagent"
version.workspace = true
```

```toml
# src-tauri/installer-actions/Cargo.toml
[package]
name = "fyagent-installer-actions"
version.workspace = true

[lib]
crate-type = ["cdylib"]
```

The only manually maintained FyAgent application-version literal is
[workspace.package].version in src-tauri/Cargo.toml. The workspace must contain
exactly the root package and installer-actions with resolver 2; both package
manifests must inherit version.workspace = true. package.json must be private
and must not declare an application version. src-tauri/tauri.conf.json must omit
version so Tauri inherits Cargo metadata.

The local Cargo.lock must contain exactly one local, source-less package block
for fyagent and fyagent-installer-actions, and both blocks must match the
workspace version.

### Version command interface

```text
pnpm run version:get
pnpm run version:check [-- --tag vX.Y.Z]
pnpm run version:set X.Y.Z [-- --apply | --dry-run]
pnpm run version:bump patch|minor|major [-- --apply | --dry-run]
```

- get prints one stable SemVer X.Y.Z with no prefix, prerelease, or build
  metadata.
- check validates the complete version contract. With --tag, it accepts only
  exactly vX.Y.Z for the canonical version.
- set performs a structural preflight and previews by default. Only --apply
  changes [workspace.package].version and the two permitted local Cargo.lock
  package versions. Each target uses a unique temporary file in the target
  directory, followed by a complete write, file fsync, close, and rename-based
  per-file atomic replacement. A controlled failure or failed post-write check
  restores every already replaced target through the same atomic replacement
  path and removes temporary files.
- The two Cargo files are not one power-loss-atomic filesystem transaction. A
  process or machine failure between the two per-file renames can leave version
  drift; version:check detects it, and a later structurally valid --apply may
  repair only the two local version values. Do not describe this boundary as a
  durable multi-file transaction.
- bump validates the existing contract first, derives patch, minor, or major,
  then uses the same preview/apply path. --dry-run is retained as an explicit
  alias for the default preview; combining it with --apply is an error. get and
  check reject --apply.
- Versions must also fit MSI ProductVersion: major and minor are at most 255;
  patch is at most 65535.

### Frozen release values

```text
version-contract outputs:
  app_version = version:get
  release_tag = "v" + app_version
  source_sha  = full GitHub commit SHA

release job environment:
  APP_VERSION = needs.version-contract.outputs.app_version
  RELEASE_TAG = needs.version-contract.outputs.release_tag
  SOURCE_SHA  = needs.version-contract.outputs.source_sha
```

The version-contract job is the only release-workflow producer of these values.
Every platform build and publication step consumes its outputs; no downstream
job may trim GITHUB_REF_NAME, reread an alternative version field, or substitute
its own source SHA.

### Windows MSI custom-action ABI

```rust
pub unsafe extern "system" fn ValidateFyAgentInstallDirUi(
    install: MSIHANDLE,
) -> u32;

pub unsafe extern "system" fn ValidateFyAgentInstallDirExecute(
    install: MSIHANDLE,
) -> u32;
```

The WiX template receives the target-specific helper through
FYAGENT_INSTALLER_ACTIONS_DLL and the Tauri-visible bridge
TAURI_FYAGENT_INSTALLER_ACTIONS_DLL. Its MSI Binary stream is
FyAgentInstallerActions. The helper uses the already locked windows-sys 0.61
family, is independent of Tauri, and must not turn the main application crate
into a cdylib.

The stable directory result properties are:

```text
FYAGENT_INSTALLDIR_VALID
FYAGENT_INSTALLDIR_ERROR_CODE
FYAGENT_INSTALLDIR_ERROR_MESSAGE
FYAGENT_INSTALLDIR_NORMALIZED
FYAGENT_INSTALLDIR_CHECK_ID
```

## 3. Contracts

### Canonical metadata and version updates

- Use the version commands only for the canonical Cargo metadata and permitted
  local Cargo.lock package blocks. They do not create Git tags, rename assets,
  change release workflow configuration, or rewrite historical documents; those
  follow their separately owned, reviewed boundaries.
- A version update must not upgrade Node, Rust, pnpm, WiX, windows-sys, or
  unrelated dependencies. Cargo.lock changes are limited to the two local
  workspace package version blocks unless a separately approved dependency
  change requires more.
- package.json scripts version:get, version:check, version:set, and
  version:bump must remain exact Node 24.19.0 entry points to
  scripts/version.mjs.
- Versioned product documents can contain historical snapshots. They are not
  application metadata and do not count as duplicate version sources.

### Release and download-manifest contract

- The GitHub tag trigger may match v\*, but a tag build enters the platform
  matrix only after version:check proves GITHUB_REF_NAME equals the release tag
  formed by a v prefix plus app_version. A prerelease or suffixed tag must fail
  before a platform build.
- A workflow_dispatch run on a branch may produce only the unsigned macOS
  developer artifact. A manual tag dispatch runs the frozen contract check but
  skips the release and publish jobs. Only a qualifying tag push can publish.
- Release assets use the unprefixed application version, for example:

  ```text
  FyAgent-X.Y.Z-macOS.dmg
  FyAgent-X.Y.Z-Windows.msi
  FyAgent-X.Y.Z-Windows-arm64.msi
  FyAgent-X.Y.Z-Linux-x86_64.AppImage
  ```

  Do not derive asset names from RELEASE_TAG and do not add a v before X.Y.Z.

- scripts/generate-download-manifest.mjs accepts assets directory,
  app version, release tag, source SHA, base URL, and optional output path. It
  rejects a non-exact tag, an invalid/full-length-missing SHA, an asset that
  does not begin with the frozen FyAgent-version prefix, and a release with no
  recognized assets. The generated manifest has schema, version, tag, sourceSha,
  pubDate, and per-asset platform/kind/arch/name/size/sha256/url fields.
- Signing, timestamping, notarization, and publication remain release gates.
  A local native build or a passing static workflow test does not establish any
  of them.

### Native MSI install-directory contract

- The policy core is shared by the UI and Execute entry points. It normalizes
  the requested DOS path and fails closed unless it can prove a fixed local
  volume, no unsafe reparse traversal, no system/user/temporary/ProgramData
  placement, a trusted existing ancestor, trusted owner and DACL, and no
  effective non-trusted write/delete/create capability.
- A non-empty target is allowed only for a verified existing FyAgent
  installation/maintenance context with a regular FyAgent marker. Validation
  does not create files, write a registry key, change an ACL, invoke WMI, or
  depend on PowerShell, VBScript, JScript, or a target-machine script.
- The UI Type 1 action records a policy rejection in the stable properties and
  returns MSI success so the directory dialog can stay recoverable and show its
  short user message. The Execute Type 1 action repeats the same policy;
  ApplyValidatedFyAgentInstallDir applies the normalized value only when valid,
  and a following Type 19 action stops the transaction before InstallValidate
  or InstallFiles when invalid.
- Maintenance clears caller-provided public INSTALLDIR and the previous-anchor
  property before AppSearch, restores only the protected HKLM InstallDir
  anchor before CostFinalize, then revalidates it. A repair or upgrade without
  that anchor must fail before file writes; it must not trust a command-line
  INSTALLDIR.
- Pure uninstall is the only directory-validation exemption. Its condition
  must exactly represent every component rooted at INSTALLDIR in the rendered
  MSI Directory and Component tables. At this baseline the closure is
  CMP_UninstallShortcut, InstallDirectoryAcl, Path, and RegistryEntries.
  Adding a direct or indirect INSTALLDIR component requires changing the WiX
  predicate and the Linux/Windows structure gates together.

### Architecture and MSI-table contract

- Build fyagent-installer-actions separately for x64 and ARM64 before each
  Tauri WiX bundle. Verify the PE Machine value, the target MSI summary
  architecture, and byte equality between the verified DLL and the embedded
  FyAgentInstallerActions Binary stream.
- The rendered MSI must contain both native Type 1 actions, the UI error
  dialog, execute Type 19 rejection paths, HKLM InstallDir search/restore
  actions, the protected final directory ACL, and the exact UI/Execute ordering.
  It must not contain the retired scripted FyAgent directory validator.
- Non-Windows compilation retains portable stubs and pure policy tests. It is
  not evidence that a non-Windows host can execute the Windows Custom Action.

## 4. Validation & Error Matrix

| Condition                                                                                                       | Required result                                                                                                               |
| --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Workspace member, resolver, inherited package version, package private flag, or duplicate metadata field drifts | version:check fails before a release or version write.                                                                        |
| Version is not stable X.Y.Z or exceeds the MSI ProductVersion bounds                                            | get, set, bump, or check fails; no release tag is accepted.                                                                   |
| Local Cargo.lock package block is missing, duplicated, sourced, or mismatched                                   | version:check fails; set may repair only the local version value after all other preflight checks pass.                       |
| A tag matches v\* but is not exactly the version-contract release tag                                           | version-contract fails before the platform matrix.                                                                            |
| A platform asset has a v-prefixed version or differs from APP_VERSION                                           | platform/release manifest validation fails; it is not published.                                                              |
| Manifest tag, source SHA, or recognized asset set is invalid                                                    | generate-download-manifest.mjs fails and publication stops.                                                                   |
| Helper DLL PE machine, MSI summary architecture, or embedded Binary bytes differ                                | Native release structure verification fails before candidate publication or signing.                                          |
| UI policy denial                                                                                                | Set valid=0 and stable error properties, show the policy dialog, and leave the user at the directory step without Error 1720. |
| Silent install or Execute policy denial                                                                         | The Execute action records the same rejection and Type 19 aborts before file installation.                                    |
| Repair/upgrade has no trusted HKLM InstallDir anchor                                                            | Type 19 stops maintenance before validation/file writes.                                                                      |
| Transaction is a true pure uninstall                                                                            | Skip directory admission only for the complete rendered INSTALLDIR component closure.                                         |
| A v1-0.3.0 package edit is untracked, or the final regenerated manifest does not match                          | Traceability/closeout fails; preserve the edit history and regenerate MANIFEST.sha256 only with final evidence.               |

## 5. Good / Base / Bad Cases

- Good: Cargo has workspace.package.version = 0.3.0, both local packages
  inherit it, and version:check with tag v0.3.0 succeeds. The tag push freezes
  app_version=0.3.0, release_tag=v0.3.0, and one source SHA before all
  platform jobs use those exact outputs.
- Good: The x64 MSI embeds the x64 helper and the ARM64 MSI embeds the ARM64
  helper. Both tables run the native policy in UI and Execute, and both fail
  maintenance safely when the HKLM anchor is absent.
- Base: pnpm run version:set X.Y.Z and the equivalent explicit --dry-run report
  only Cargo.toml and local Cargo.lock changes without writing. A branch
  workflow_dispatch produces the explicitly
  unsigned macOS artifact but never creates or updates a GitHub Release.
- Bad: Add a version property back to package.json, use GITHUB_REF_NAME as the
  platform version, trim v from a tag in a platform job, or hand-edit a
  historical document as part of a bump.
- Bad: Reintroduce a WMI/VBScript directory check, accept a UI-only validation,
  embed one architecture's DLL in the other MSI, accept an attacker-supplied
  repair INSTALLDIR, or skip validation for a mixed remove-and-install
  transaction.

## 6. Tests Required

- Version tests must cover get/check/set/bump, stable SemVer and MSI bounds,
  dry-run, preflight failure, per-file atomic replacement, cleanup after a
  partially written temporary file, rollback after a later replacement or
  post-write verification failure, duplicate version fields, missing workspace
  member/script/local lock block, and non-exact tags.
- tests/versionConsistency.test.ts must delegate to the canonical script.
  tests/downloadManifest.test.ts must assert frozen version/tag/source SHA,
  unprefixed asset names, URL shape, and invalid-input rejection.
- tests/releaseWorkflow.test.ts must assert the version-contract job,
  downstream output consumption, exact tag validation, helper build ordering,
  Type 1/Type 19 actions, MSI component closure protection, and absence of the
  retired directory script.
- The helper unit tests must cover portable policy cases. Windows-only policy
  and ACL integration tests run on Windows and must not be represented as
  passing on another host.
- Local checks, when their host prerequisites are available:

  ```bash
  mise exec -- pnpm typecheck
  mise exec -- pnpm format:check
  mise exec -- pnpm test:unit
  mise exec -- node --test tests/version.test.mjs
  mise exec -- cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
  mise exec -- cargo clippy --workspace --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
  mise exec -- cargo test --workspace --manifest-path src-tauri/Cargo.toml
  app_version="$(mise exec -- pnpm --silent run version:get)"
  mise exec -- pnpm run version:check -- --tag "v$app_version"
  mise exec -- pnpm vitest run tests/localBuildBoundary.test.ts tests/releaseWorkflow.test.ts
  # Child 6 closeout, after the final audited MANIFEST.sha256 regeneration:
  (cd docs/fyagent/dev/v1-0.3.0 && sha256sum -c MANIFEST.sha256)
  ```

- Native release evidence remains separate: Windows x64 and ARM64 must cover
  default, safe custom, unsafe custom, /qn INSTALLDIR, upgrade, repair,
  uninstall, verbose MSI log, and ICE behavior. Signing/timestamping, macOS
  notarization, final multi-platform metadata, published manifest, exact tag,
  and source SHA require an explicitly authorized release run.

## 7. Wrong vs Correct

### Wrong

```text
package.json.version = "0.2.2"
tauri.conf.json.version = "0.2.2"
GITHUB_REF_NAME is stripped and used as every platform's asset version
one x64 helper DLL is reused for every MSI
```

This creates competing sources, lets a broad tag filter become a release
identity, and makes MSI architecture/policy verification accidental.

### Correct

```bash
pnpm run version:set 0.3.0 -- --apply
pnpm run version:check -- --tag v0.3.0
```

Then let version-contract freeze its three outputs, pass APP_VERSION,
RELEASE_TAG, and SOURCE_SHA unchanged to every platform, build a matching
installer-actions DLL per target, and retain the UI-plus-Execute native
directory-policy sequence.
