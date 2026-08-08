# GitHub Release Workflow Contract

## 1. Scope / Trigger

This contract applies to `.github/workflows/release.yml`, its native platform
jobs, release evidence generators, artifact attestations, and the one-time
GitHub Release publication step for FyAgent 0.3.0.

The workflow supports exactly two entry modes:

```yaml
on:
  push:
    tags:
      - "v0.3.0"
  workflow_dispatch:
    inputs:
      source_sha:
        required: true
```

- `workflow_dispatch` is an unsigned, full five-target-group preflight for the
  exact trusted `main` workflow commit. Its lowercase 40-character
  `source_sha`, `GITHUB_SHA`, and `GITHUB_WORKFLOW_SHA` must be identical so
  the standard GitHub attestation provenance describes the bytes actually
  built. It produces workflow artifacts and attestations but never creates or
  updates a GitHub Release.
- a push of the exact `v0.3.0` tag is the only formal path. The tag, product
  version, workflow ref, event SHA, checked-out commit, `origin/main` ancestry,
  and successful same-SHA Required CI must all agree.
- no branch push, broad `v*` tag, manual signed mode, manual tag dispatch,
  partial platform mode, or local publish path exists.

The post-merge exact-main-SHA sequence above is implemented in the workflow
and locally verified. The project owner accepted D113/D114 on 2026-08-08;
[D113](../../../docs/fyagent/dev/v1-0.3.0/decisions/DECISION-REGISTER.md)
confirms this ordering. That decision is not evidence that a preflight ran or
that a tag, Release, asset, or attestation exists.

Local implementation and static tests do not authorize dispatch, tag creation,
or publication. Remote preflight, formal Release, and post-publication evidence
remain required before the owning Trellis task can close.

When a release run is explicitly authorized, the initiating main flow waits
synchronously for that entire run to reach `completed`. It does not delegate
monitoring to a background or asynchronous agent and does not repeatedly poll
run state. After completion it reads the run result once; only a failed result
authorizes one retrieval of the failed-job logs. This observation contract does
not itself authorize dispatch, rerun, cancellation, tag creation, or publish.

## 2. Frozen Values and Job Topology

```text
eligibility
  app_version = 0.3.0
  release_tag = v0.3.0
  source_sha = immutable main commit
  release_mode = preflight | formal
  ci_run_id / ci_run_attempt = exact successful main push CI (formal only)

eligibility ─┬─> build-windows (windows-x64, windows-arm64) ─┐
             ├─> build-linux   (linux-x64, linux-arm64) ─────┼─> verify-assets
             └─> build-macos   (macos-universal) ────────────┘

verify-assets -> attest -> publish (formal only)
```

Every platform job receives the same values only from `eligibility`. It checks
out `source_sha` directly, validates the product tag through `version:check`,
and records the trusted workflow SHA; formal metadata also records the selected
Required CI run, while preflight records `requiredCi: null`. Platform jobs must
not derive a version from a ref, package.json,
Tauri configuration, a bundle filename, or a second version source.

## 3. Eligibility Contract

Eligibility fails closed unless all of the following are true:

1. `GITHUB_REPOSITORY` is `NongHua123/fyagent` and
   `GITHUB_REPOSITORY_ID` is `1313497021`.
2. before checkout, the request envelope proves the executing workflow is
   `Release` at `.github/workflows/release.yml` and accepts only trusted
   `refs/heads/main` dispatch or exact `refs/tags/v0.3.0` push.
3. dispatch requires `source_sha == GITHUB_SHA == GITHUB_WORKFLOW_SHA`; formal
   requires workflow/event/tag/candidate commits to peel to the same source.
4. the trusted `scripts/version.mjs` is copied into an isolated temporary tree
   and reads candidate files as data; eligibility never installs dependencies
   or executes a candidate version script.
5. a fresh fetch proves the trusted workflow SHA is on `origin/main`; formal
   additionally proves `source_sha` is an `origin/main` ancestor.
6. formal mode alone requires the active CI workflow identity to be exactly
   `.github/workflows/ci.yml`.
7. in formal mode, among main push CI runs for the exact source, the latest run/attempt is
   completed successfully. An older success cannot mask a newer failure,
   cancellation, or in-progress attempt.
8. the formal selected attempt contains exactly one completed/successful
   `CI / Required` job.
9. its check suite contains exactly one matching `CI / Required` check-run from
   the `github-actions` app whose head SHA, API job/check URL, and details URL
   are bound to that selected run and job.

The approved pre-merge preflight order cannot be represented truthfully by
standard `actions/attest` in this one-workflow design: dispatch provenance is
bound to the workflow `GITHUB_SHA`, not to a different unmerged candidate.
v0.3.0 therefore runs its first preflight after merge on the exact `main` SHA.
A future unmerged-candidate design requires a separate trusted reusable
workflow or custom predicate and is outside this release.

This is the implemented and accepted D113 sequence. The 2026-08-08 project
decision clears only the ordering question in the
[decision register](../../../docs/fyagent/dev/v1-0.3.0/decisions/DECISION-REGISTER.md);
remote preflight, exact tag creation, formal publication, asset checks,
attestations, and closeout remain separate pending evidence.

This workflow-only admission is intentionally weaker than administrator-backed
branch/tag rulesets or a protected environment. FyAgent 0.3.0 accepts that
residual supply-chain risk; the repository must not claim that main or the tag
is administrator-protected.

## 4. Runner, Toolchain, and Build Contract

Direct third-party Actions use reviewed full 40-character commit SHAs. Required
Release jobs do not use `*-latest` runners. Actions do not install or execute
mise. Release jobs do not restore or save dependency caches; candidate build
code cannot populate a trusted-main cache later consumed by formal release.
The native build jobs establish the repository-declared Node version before
running non-standalone `pnpm/action-setup`, whose installer requires `npm` on
`PATH`; relying on a runner image's incidental npm installation is invalid.
Both pnpm setup and `setup-rust-toolchain` declare `cache: false` explicitly.
The Rust action enables `Swatinem/rust-cache` by default when that input is
omitted, so absence of the field is not evidence that Release caching is off.

| Target group      | Runner             | Build user space                       | Required output                    |
| ----------------- | ------------------ | -------------------------------------- | ---------------------------------- |
| `windows-x64`     | `windows-2022`     | native x64                             | one x64 MSI                        |
| `windows-arm64`   | `windows-11-arm`   | native ARM64                           | one ARM64 MSI                      |
| `linux-x64`       | `ubuntu-24.04`     | native Ubuntu 22.04 amd64 child digest | AppImage, DEB, RPM                 |
| `linux-arm64`     | `ubuntu-24.04-arm` | native Ubuntu 22.04 arm64 child digest | AppImage, DEB, RPM                 |
| `macos-universal` | `macos-15`         | macOS with both Apple targets          | DMG and ZIP from one universal app |

Linux uses the reviewed, fully qualified Ubuntu 22.04 image children directly:

```text
amd64 docker.io/library/ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e
arm64 docker.io/library/ubuntu:22.04@sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149
```

The workflow verifies `RUNNER_ARCH`, `/etc/os-release`, and `uname -m` before
building, so a wrong host plus binfmt cannot impersonate a native target. There
is no QEMU or opposite-architecture fallback. ARM runner unavailability is a
retryable infrastructure failure, not authorization to cross-build or publish
a reduced asset set.

GitHub job containers restore their ordinary `HOME` after checkout has written
its temporary global Git configuration. Each Linux build therefore clears any
inherited global `safe.directory` values, adds only the exact
`$GITHUB_WORKSPACE` path, proves that it is the sole value visible across the
effective configuration scopes, and immediately proves its HEAD equals the
frozen source SHA. Wildcard or additional safe-directory trust, recursive
ownership changes, or disabling Git's ownership check are forbidden.

Each target proves Node 24.19.0, pnpm 10.12.3, and Rust 1.97.1 at runtime. Every
`fyagent-platform-build/v1` record uses one source-explicit shape:

- `runner.requestedLabel` is the exact matrix routing request; it is not a
  runtime-discovered host label or immutable hosted-image identity.
- `runner.context.os` and `runner.context.arch` come only from documented
  `${{ runner.os }}` and `${{ runner.arch }}` values mapped into the
  workflow-owned `ACTUAL_RUNNER_OS` and `ACTUAL_RUNNER_ARCH` variables.
  `windows-x64` requires `Windows` / `X64`, `windows-arm64` requires `Windows`
  / `ARM64`, `macos-universal` requires `macOS` / `ARM64`, and both Linux
  targets use the exact pairs below. The macOS output architecture remains
  `universal`; that output fact is distinct from, and does not weaken, the
  current `macos-15` hosted-runner architecture contract.
- Windows and macOS record exactly `container: null` and reject any supplied
  container evidence.
- Linux records the configured
  `container.configuredImage.reference` and `.manifestDigest` from the exact
  matrix image request, plus emission-time observations in
  `container.observed.osRelease.id`, `.versionId`, and `.unameMachine`.
  `linux-x64` requires `ubuntu` / `22.04` / `x86_64` with the amd64 reference
  above; `linux-arm64` requires `ubuntu` / `22.04` / `aarch64` with the arm64
  reference above.

The Linux metadata step repeats the runner-context, `/etc/os-release`, and
`uname -m` gates immediately before invoking the writer. This late measurement
is distinct from the early bootstrap gate: the first prevents expensive work
in the wrong environment, while the second supplies the observations that are
actually serialized. The writer never reads ambient `RUNNER_OS`,
`RUNNER_ARCH`, `ImageOS`, or `ImageVersion`; the latter two implementation
details are removed rather than retained as nullable compatibility fields.
Missing, blank, partial, contradictory, or malformed owned evidence fails.

The configured image reference is reviewed workflow configuration, not a
digest independently measured from inside the container. `/etc/os-release`,
`uname -m`, and the artifact attestation corroborate user-space, machine, bytes,
and workflow provenance, but none independently proves the configured OCI
digest or certifies the semantic truth of arbitrary custom JSON. The metadata
therefore contains no `verified` boolean, fabricated actual-image digest, or
guessed hosted-image version.

The locked `@tauri-apps/cli` 2.8.1 embeds `tauri-bundler` 2.6.1, before the
nested AppImage-plugin propagation fixed by `tauri-apps/tauri#14241`. The Linux
package step alone therefore exports `APPIMAGE_EXTRACT_AND_RUN=1` and invokes
Tauri with `--verbose`. This keeps nested `linuxdeploy` execution in extraction
mode on the unprivileged container; it does not add `SYS_ADMIN`, privileged
container mode, a `/dev/fuse` device, or any other mount capability. The
workaround must be removed or revalidated when the locked Tauri CLI/bundler is
upgraded.

## 5. Platform Security Gates

### Windows

- both native jobs set `FYAGENT_WINDOWS_MANIFEST=release` on the application
  build and MSI bundle commands.
- the MSI bundle command uses `--verbose`, captures `$LASTEXITCODE` immediately,
  and fails before looking for an MSI when Tauri exits nonzero. Candle/Light
  stderr remains visible, and Light runs its normal ICE validation without
  `-sval` or individual ICE suppression.
- the application executable is inspected before and after bundling for exact
  x64/ARM64 PE Machine, `requireAdministrator`, `uiAccess=false`, bundle
  version, and exactly one `requestedExecutionLevel`.
- the architecture-matched installer-actions DLL is built separately, checked
  for PE Machine, and supplied through both helper environment variables.
- `verify-windows-msi-structure.ps1` preserves the Type 1/Type 19 actions, the
  post-`CostFinalize` Type 35 normalized-directory assignment, HKLM anchor,
  protected DACL, native complete `INSTALLDIR` component classifier,
  context-redirected machine-wide Desktop/Programs shortcuts, UI/Execute sequence,
  unsafe-directory dialog, and MSI summary architecture gates formerly inlined
  in the workflow. After `CostFinalize`, one Type 1 classifier runs independently
  in each MSI sequence, clears the private mixed-case `FyAgentPureUninstall`
  marker, queries the active MSI Directory and Component tables, and sets the
  marker only when every actual `INSTALLDIR` descendant has action state
  `INSTALLSTATE_ABSENT`. It rejects empty, duplicate, over-limit, malformed, or
  missing-core closures and fails the transaction closed on any MSI API error.
  This includes generated resources, bundled binaries, and conditional update
  components without putting an over-limit component expression in the MSI
  Sequence Condition column. The verifier independently derives the real table
  closure, requires the four core components, checks the private marker has no
  authored default, and proves classifier ordering before every consumer. The
  per-machine package keeps the standard `DesktopFolder` and
  `ProgramMenuFolder` identifiers, which `ALLUSERS=1` redirects to All Users.
  Both shortcut rows are advertised-authored children of the existing `Path`
  file component, while `DISABLEADVTSHORTCUTS=1` makes Windows Installer emit
  ordinary shortcuts; no profile-scoped shortcut component, marker KeyPath, or
  explicit shortcut Icon exists. The verifier requires both rendered targets
  to use the same Feature, proves `FeatureComponents` binds it to `Path`, and
  keeps `RemoveShortcuts` before the one product-folder `RemoveFiles` cleanup.
  It also reads the embedded cabinet stream through the
  read-only MSI database, extracts only fixed File key `Path` with system
  `expand.exe` into a fresh root, and binds the final MSI executable to the
  already verified built executable by size, SHA-256, PE Machine, and
  Authenticode `NotSigned` without executing the installer.
- `verify-windows-msi.ps1` independently verifies the embedded Binary stream,
  helper SHA/PE identity, product/version/repair properties, protocol registry,
  single `fyagent.exe` payload, architecture, and absence of retired host-path
  residue.
- both the executable and MSI must report Authenticode `NotSigned` with no
  signer or timestamp certificate. No Windows certificate secret or signing
  command belongs in v0.3.0.

### macOS

- one `universal-apple-darwin` app must contain both `arm64` and `x86_64`
  slices, version 0.3.0, and bundle identifier `com.fyagent.desktop`.
- a truly unsigned app or ad-hoc signature is acceptable. A Developer ID
  Authority or real TeamIdentifier is forbidden.
- the app and DMG must not validate as stapled/notarized. The workflow may run
  negative `stapler validate` checks; it must never run `stapler staple`,
  `notarytool`, or a signing secret path.
- ZIP and DMG are created from the same app. The ZIP is expanded, the DMG is
  verified and mounted read-only, and both copies must retain the app version
  and executable SHA-256.

### Linux

- each native container must produce exactly one raw AppImage, DEB, and RPM.
- the package step uses the step-scoped extraction-mode compatibility variable
  and `--verbose`; later validation/normalization steps do not inherit it.
- AppImage ELF architecture, DEB version/architecture, and RPM
  version/architecture must match the frozen target before normalization.
- missing formats are failures; no format is optional in the formal or
  preflight matrix.

## 6. Asset, Manifest, Metadata, and Attestation Contract

The installer allowlist contains exactly ten files:

```text
FyAgent-0.3.0-macOS.dmg
FyAgent-0.3.0-macOS.zip
FyAgent-0.3.0-Windows.msi
FyAgent-0.3.0-Windows-arm64.msi
FyAgent-0.3.0-Linux-x86_64.AppImage
FyAgent-0.3.0-Linux-x86_64.deb
FyAgent-0.3.0-Linux-x86_64.rpm
FyAgent-0.3.0-Linux-arm64.AppImage
FyAgent-0.3.0-Linux-arm64.deb
FyAgent-0.3.0-Linux-arm64.rpm
```

Every platform artifact remains in its named directory until
`collect-workflow-artifacts.mjs` validates the expected artifact tree. The
collector refuses missing, extra, misplaced, nested, symlinked, or duplicate
files and copies with no-overwrite semantics. Flattening in the download Action
must not mask a duplicate.

`generate-download-manifest.mjs` then requires the exact ten non-empty
installers and emits `download-manifest.json` schema
`fyagent-download-manifest/v2`. It records product, version, tag, source SHA,
publication instant, and each installer's name, platform, architecture, format,
size, SHA-256, and final URL.

`generate-build-metadata.mjs` requires exactly five platform metadata records.
It validates target/runner/container identity, repository ID, trusted workflow
ref/SHA/run, release mode, source, and exact runner/toolchain evidence before
emitting `build-metadata.json`. Every input object uses an exact key allowlist
at the record, runner, runner-context, container, configured-image,
observation, OS-release, toolchain, and identity levels. Unknown or retired
keys fail; after validation the aggregate reconstructs each target from the
allowlist instead of spreading parsed input. `requiredCi` is `null` for
preflight and the unique bound path/run/attempt object for formal mode.

Local and read-only release evidence shows that neither draft metadata schema
has been publicly released or consumed, so this change finalizes
`fyagent-platform-build/v1` and `fyagent-build-metadata/v1` in place before
their first publication. If any public v1 consumer is discovered before that
publication, both identifiers and all writers/validators/types/tests/docs must
move atomically to v2; the formal path then accepts only v2. There is no v1
compatibility reader, defaulting path, or synthesized equivalence.

The attestation subjects are exactly the ten installers plus those two JSON
files (12 subjects). `actions/attest` v4.2.2 is mandatory and receives only
those files. Its Sigstore bundle is copied to the fixed independent name
`artifact-attestation.sigstore.json`, producing exactly 13 allowed Release
attachments. The bundle is evidence and does not count as an installer.

## 7. Permission and Publication Transaction

```yaml
permissions:
  contents: read
```

- eligibility alone adds `actions: read` and `checks: read`.
- attestation alone adds `id-token: write`, `attestations: write`, and
  `artifact-metadata: write`.
- publish alone adds `contents: write` after eligibility, all native builds,
  exact-asset verification, evidence generation, and attestation succeed.
- v0.3.0 uses no Release environment, signing credential, environment approval,
  or signed mode.

Publish rechecks the exact formal event/tag/source and the 13-file allowlist,
requires the English v0.3.0 Release Notes, and uses the authenticated Release
list (including drafts) to fail if any `v0.3.0` Release already exists. It then
creates one private draft carrying a run/source ownership marker, uploads the
13 files, lists and re-downloads them, proves exact names/non-empty states and
SHA-256 equality, then re-reads the draft ID/tag/marker/state and exact asset
IDs immediately before one final PATCH to stable/non-prerelease/latest. A
successful PATCH response is not sufficient: publish re-reads the Release by
ID, verifies the published identity and exact asset IDs, and confirms the
latest Release before declaring the transaction complete.
No failure path automatically deletes the draft: Release DELETE has no atomic
conditional guard, so deletion could race a concurrent publication. A failed
transaction reports its Release ID/URL for a separate manual decision. Once a
PATCH has been attempted, its exit handler performs one read-only API lookup
and reports the observed state as draft, published, or unknown; it never
retries PATCH and never claims the Release remains private when the outcome is
ambiguous. Any retry fails closed while that draft or published Release exists.
The workflow never updates an existing Release or moves/deletes the tag.
Because GitHub does not offer a general conditional guard for this unsafe
PATCH, an administrator could still race the final read/PATCH; that narrow
workflow-only residual risk is accepted alongside the absence of repository
rulesets and is not described as atomic administrator protection.

## 8. Failure Matrix

| Condition                                                                                                                     | Required result                                                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Dispatch SHA is not full/lowercase or differs from trusted main workflow/event provenance                                     | Fail eligibility before any platform build.                                                                         |
| Formal ref, workflow ref, tag commit, event commit, product version, or source differ                                         | Fail eligibility; do not build or publish.                                                                          |
| Latest same-SHA main CI attempt is absent, running, failed, cancelled, or lacks the unique Required job/check                 | Fail eligibility; an older success is not accepted.                                                                 |
| A native runner/architecture, Ubuntu child digest, or tool version drifts                                                     | Fail that platform job; no fallback target is allowed.                                                              |
| Node is not established before pnpm, a Linux container lacks exact workspace trust, or a Release cache is enabled             | Fail platform bootstrap; do not rely on runner-preinstalled tools, wildcard Git trust, or implicit Action defaults. |
| Windows bundle exits nonzero, Light reports an ICE error, or a manifest/helper/MSI structure/payload/unsigned assertion fails | Preserve verbose stderr and fail Windows output before MSI enumeration or artifact upload.                          |
| macOS app is not universal, identity differs, distribution identity/ticket exists, or ZIP/DMG copies differ                   | Fail macOS output before artifact upload.                                                                           |
| Linux nested AppImage execution or package count/version/architecture differs                                                 | Fail Linux output with verbose downstream stderr before artifact upload; do not add mount privileges.               |
| Artifact tree or exact ten/twelve/thirteen allowlist differs                                                                  | Fail verification/attestation/publish.                                                                              |
| Mandatory attestation or bundle is absent                                                                                     | Fail; do not characterize hashes alone as v0.3.0 provenance success.                                                |
| Dispatch reaches publish                                                                                                      | Static workflow test fails; remote preflight must create no Release.                                                |
| A draft or published Release already exists                                                                                   | Refuse to update, replace, or delete it.                                                                            |
| Upload/re-download fails before final PATCH                                                                                   | Leave the draft untouched, report ID/URL, and require manual decision.                                              |
| Final PATCH has a failed or ambiguous outcome                                                                                 | Read state by ID, report draft/published/unknown, and never retry or delete.                                        |

## 9. Validation and Evidence Boundary

Local checks include Prettier, actionlint, version contract tests, the release
workflow/static Windows boundary suite, download-manifest behavior tests,
asset/metadata collector tests, and `tests/writePlatformMetadata.test.ts`. The
writer suite invokes the real CLI for all five targets and covers missing,
blank, partial, extra, contradictory, malformed, existing-output, hostile
ambient-variable, and writer-to-aggregate cases. Aggregate tests reject unknown
keys at every nested input level and prove canonical output reconstruction.
Local execution is restricted to the current host OS and architecture. A
subsystem bridge, foreign executable, cross target, emulator, or locally copied
non-host toolchain cannot establish native release evidence. PowerShell
runtime, Windows Candle/Light/MSI, Linux package, macOS bundle, and every
non-host architecture check run only in their matching native GitHub Actions
jobs. No local cross-OS or cross-architecture result counts toward acceptance.

A green local suite proves the implementation contract, not publication.
Closure requires the exact source's main `CI / Required`, one successful
post-merge same-SHA full-matrix unsigned preflight, the tag-triggered formal run, the public stable
Release, independent re-download/digest/unsigned-state checks, attestation evidence,
and audited Trellis closeout records. D113 has explicit project acceptance,
but local contract success and that decision alone cannot satisfy any of these
remote Release gates.
