# GitHub Release Workflow and Frozen Version Contract

## 1. Scope / Trigger

This contract applies to .github/workflows/release.yml and its user-facing
assets. It separates an explicitly manual, branch-only unsigned macOS developer
artifact from the signed GitHub prerelease tag path while requiring all platform
jobs to share one frozen application-version identity.

Read [FyAgent 0.2.1 Version and Installer Contract](./fyagent-version-contract.md)
before changing a version command, tag rule, release asset name, manifest
generator, Windows MSI, or platform bundle metadata. This document owns the
workflow scheduling and release-provenance behavior; it does not authorize
remote execution, signing, notarization, tagging, or publication from a local
documentation update.

## 2. Signatures

```yaml
on:
  push:
    tags:
      - "v*"
  workflow_dispatch:

jobs:
  version-contract:
    outputs:
      app_version:
      release_tag:
      source_sha:
  release:
    needs: version-contract
  publish-release:
    needs: [version-contract, release]
```

```text
app_version = pnpm run version:get
release_tag = "v" + app_version
source_sha  = full GITHUB_SHA

tag push (only after exact validation):
  Windows x64, Windows ARM64, Linux x64, Linux ARM64, macOS

workflow_dispatch on a branch:
  macOS only, unsigned workflow artifact

workflow_dispatch on a tag:
  version-contract may validate the ref; release and publish-release skip
```

All platform jobs receive:

```text
APP_VERSION = needs.version-contract.outputs.app_version
RELEASE_TAG = needs.version-contract.outputs.release_tag
SOURCE_SHA  = needs.version-contract.outputs.source_sha
```

Asset names use APP_VERSION without a v prefix:

```text
FyAgent-X.Y.Z-macOS.dmg
FyAgent-X.Y.Z-macOS.zip
FyAgent-X.Y.Z-Windows.msi
FyAgent-X.Y.Z-Windows-arm64.msi
FyAgent-X.Y.Z-Linux-x86_64.AppImage
FyAgent-X.Y.Z-Linux-arm64.AppImage
FyAgent-X.Y.Z-Linux-x86_64.deb
FyAgent-X.Y.Z-Linux-arm64.deb
FyAgent-X.Y.Z-Linux-x86_64.rpm
FyAgent-X.Y.Z-Linux-arm64.rpm
```

The branch-only macOS developer artifact uses the frozen application version:

```text
FyAgent-X.Y.Z-macOS-unsigned.dmg
```

## 3. Contracts

### Freeze before every platform build

- version-contract runs first and invokes version:check before version:get.
  On a tag ref it invokes version:check with the actual tag and compares it
  exactly with the v-prefixed canonical app version.
- The v\* workflow filter is only routing. A tag that matches it but contains a
  prerelease, suffix, wrong number, or other mismatch must fail in
  version-contract before release matrix work begins.
- release and publish-release consume the three outputs unchanged. A platform
  step must not use GITHUB_REF_NAME, trim a tag, read a second application
  version field, or construct a substitute source SHA.
- Version commands in CI must stay compatible with Node 20. Do not infer a
  toolchain upgrade from the local mise Node declaration.

### Platform outputs and provenance

- Tauri, macOS bundle metadata, Windows executable/MSI metadata, Linux package
  metadata, and every formal asset filename must equal APP_VERSION where their
  platform representation permits it.
- The release download manifest is generated only after platform assets pass
  their platform gates. The generator receives the frozen app version, tag, and
  source SHA as explicit arguments; it never derives version by stripping a
  tag.
- The manifest schema includes version, tag, sourceSha, pubDate, and asset
  records containing platform, kind, architecture, filename, size, SHA-256,
  and URL. It rejects an invalid/full-length-missing source SHA, a non-exact
  tag, a release with no recognized assets, or a recognized asset whose name lacks the
  frozen FyAgent-version prefix.
- The tag's visible GitHub Release name uses RELEASE_TAG, while downloaded file
  names use APP_VERSION. Do not add legacy v-prefixed aliases in this workflow
  unless a separately approved compatibility change requires them.

### Branch developer artifact and formal release boundary

- Ordinary branch pushes must not trigger this workflow. A branch artifact is
  produced only when a maintainer starts workflow_dispatch on that branch.
- The branch job is macos-14 only, creates an unsigned universal app with
  bundles app, then makes a UDZO DMG. Its filename uses APP_VERSION and must
  contain unsigned.
- Before branch artifact upload, hdiutil verify, read-only attachment, a
  top-level FyAgent.app check, deterministic detach, and cleanup are required.
  Upload uses if-no-files-found: error. The branch path never creates or updates
  a GitHub Release.
- A manual tag dispatch skips both release and publish-release; it is not a
  substitute for the signed tag-push release path.
- A tag push requires the existing Apple signing/notarization inputs and fails
  closed when code signing, notarization, stapling, codesign, spctl, or
  Gatekeeper validation fails. Windows executable and MSI signing/timestamp
  checks remain mandatory on the Windows release path.
- publish-release may run only for the qualified tag-push condition after both
  version-contract and the complete platform matrix have succeeded. Its current
  GitHub Release is a prerelease; promotion to a non-prerelease is a separate
  workflow/product decision that must update this contract.
- The branch path must not execute a signing or notarization step that
  references Apple secret inputs. The shared release environment alone is not
  evidence of stronger secret isolation, so do not make an unverified
  environment-access claim in this contract.

## 4. Validation & Error Matrix

| Condition                                                                                       | Required result                                                                        |
| ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Tag matches v\* but is not the exact v-plus-canonical-version form                              | version-contract fails before a platform build.                                        |
| Version command check fails or version:get is not stable SemVer                                 | Stop before freezing outputs.                                                          |
| A downstream job uses GITHUB_REF_NAME as a version or does not receive all three frozen outputs | Static workflow test fails; reject the workflow change.                                |
| Platform metadata or formal asset name differs from APP_VERSION                                 | The relevant platform verification fails before artifact upload/publish.               |
| Asset has a v-prefixed app version or fails the recognized filename rules                       | Download-manifest generation fails; do not publish it.                                 |
| Manifest tag/source SHA does not match the frozen inputs                                        | Manifest generation fails before GitHub Release upload.                                |
| Branch DMG cannot verify, mount read-only, contain FyAgent.app, detach, or clean up             | Fail the workflow step; do not upload.                                                 |
| Apple inputs are absent on a branch dispatch                                                    | Continue only because no branch step executes a signing/notarization secret reference. |
| Apple secrets/signature/notarization are absent or invalid on a qualifying tag push             | Fail the signed release path.                                                          |
| workflow_dispatch targets any tag                                                               | Skip release and publish-release; no formal release is created.                        |
| Windows signing, timestamp, native MSI structure, or final manifest validation fails            | Stop before formal publication.                                                        |

## 5. Good / Base / Bad Cases

- Good: A push of v0.2.1 reaches version-contract, freezes app_version=0.2.1,
  release_tag=v0.2.1, and one source SHA, then all five platforms build and
  publish the GitHub prerelease assets named with 0.2.1 after their own gates
  succeed.
- Good: A maintainer manually dispatches feature/fyagent-v1 and receives only
  FyAgent-0.2.1-macOS-unsigned.dmg as a verified workflow artifact.
- Base: A maintainer manually dispatches an exact tag. version-contract validates
  the tag, while release and publish-release remain skipped; no signing or
  GitHub Release side effect occurs.
- Bad: A v0.2.1-rc tag enters a platform build because the v\* filter matched,
  a platform uses a tag-derived version, or a manifest calls an asset
  FyAgent-v0.2.1-Windows.msi.
- Bad: A branch artifact executes a signing/notarization step that references
  Apple secrets, is presented as a signed formal release, skips read-only DMG
  validation, reaches softprops/action-gh-release, or a manual tag dispatch
  enters the signed matrix.

## 6. Tests Required

- Run Prettier for the workflow and related JavaScript/TypeScript. Run
  actionlint for release.yml when it is available; if it is not installed, report
  that gap rather than characterizing the workflow as actionlint-validated.
- tests/releaseWorkflow.test.ts currently covers workflow triggers/matrices,
  frozen version outputs, helper/MSI structure gates, and the ordering of
  target-executable manifest checks. Before this workflow contract is called
  fully test-enforced, extend it to assert the APP_VERSION unsigned branch-DMG
  name and hdiutil lifecycle, GitHub prerelease=true, and the explicit
  distinction between the post-bundle target EXE check and final MSI-payload
  verification.
- tests/downloadManifest.test.ts must prove the explicit version/tag/source-SHA
  contract, recognized asset metadata, unprefixed filename rule, and rejection
  cases.
- Run the version command tests and check the current canonical tag:

  ```bash
  mise exec -- node --test tests/version.test.mjs
  app_version="$(mise exec -- pnpm --silent run version:get)"
  mise exec -- pnpm run version:check -- --tag "v$app_version"
  mise exec -- pnpm exec vitest run tests/releaseWorkflow.test.ts tests/downloadManifest.test.ts
  ```

- A successful local static test is not release evidence. Formal acceptance
  requires the authorized GitHub workflow on the exact candidate SHA and
  inspection of signed/notarized assets, asset metadata, generated manifest,
  tag, and source SHA. Native Windows MSI lifecycle testing remains a separate
  gate.

## 7. Wrong vs Correct

### Wrong

```bash
VERSION="$GITHUB_REF_NAME"
asset="FyAgent-$VERSION-Windows.msi"
```

This lets a broad trigger string become an asset version and produces a
v-prefixed filename that disagrees with the application metadata.

### Correct

```text
version-contract:
  app_version = canonical Cargo version
  release_tag = v + app_version
  source_sha = GITHUB_SHA

every platform:
  APP_VERSION, RELEASE_TAG, SOURCE_SHA = version-contract outputs
```

Validate the tag exactly before building, use APP_VERSION for asset and platform
metadata, and reserve RELEASE_TAG for the Git tag and release identity.
