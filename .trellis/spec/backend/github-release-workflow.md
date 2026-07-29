# GitHub Release Workflow Contract

## 1. Scope / Trigger

This contract applies to `.github/workflows/release.yml`. It separates a
branch-only macOS build artifact from the signed public release path so a
developer build can run without Apple credentials without weakening the
requirements for a `v*` release.

## 2. Signatures

The workflow entry points and outputs are:

    push branch: feature/fyagent-v1 -> macos-14 only
    push tag: v* -> Windows x64/ARM64, Linux x64/ARM64, macOS
    branch DMG: FyAgent-<safe-ref>-macOS-unsigned.dmg
    tag DMG: FyAgent-<tag>-macOS.dmg

Only `refs/tags/v*` may run `publish-release`.

## 3. Contracts

- A branch build must not read `APPLE_CERTIFICATE`,
  `APPLE_CERTIFICATE_PASSWORD`, `KEYCHAIN_PASSWORD`, `APPLE_ID`,
  `APPLE_PASSWORD`, or `APPLE_TEAM_ID`.
- A tag build requires those Apple secrets and fails closed when signing or
  notarization cannot be completed.
- A branch build creates an unsigned universal app with `--bundles app`, then
  creates a UDZO DMG with `hdiutil`. The filename must contain `unsigned`.
- Before upload, the branch DMG must pass `hdiutil verify`, mount read-only,
  contain top-level `FyAgent.app`, and detach successfully.
- `actions/upload-artifact` must use `if-no-files-found: error`. Branch builds
  never create or update a GitHub Release.

## 4. Validation & Error Matrix

| Condition                                    | Required result                                  |
| -------------------------------------------- | ------------------------------------------------ |
| Branch ref contains `/`                      | Replace unsupported filename characters with `-` |
| Safe branch name is empty                    | Fail before creating an asset                    |
| Universal app is absent                      | Fail before creating the DMG                     |
| DMG create or checksum verification fails    | Fail and clean temporary paths                   |
| DMG cannot mount or lacks `FyAgent.app`      | Fail, attempt detach, and clean temporary paths  |
| Detach or cleanup fails                      | Fail the workflow step                           |
| Apple secret is absent on a `v*` tag         | Fail the signed release path                     |
| Apple secret is absent on the feature branch | Continue through the unsigned path               |

## 5. Good / Base / Bad Cases

- Good: `feature/fyagent-v1` produces
  `FyAgent-feature-fyagent-v1-macOS-unsigned.dmg`, verifies and mounts it, and
  uploads it only as a workflow artifact.
- Base: a `v3.18.0` tag preserves the five-platform matrix and publishes only
  after the existing Developer ID signing, notarization, stapling, `codesign`,
  `spctl`, and Gatekeeper checks pass.
- Bad: a branch artifact is named like a signed release, reads Apple secrets,
  skips mount verification, or reaches `softprops/action-gh-release`.

## 6. Tests Required

- Run `actionlint .github/workflows/release.yml` and Prettier.
- Parse the workflow and assert the branch and tag matrices separately.
- Assert every Apple signing/notarization step is tag-only.
- Assert the branch path contains the unsigned filename, `hdiutil verify`, a
  read-only attach, the `FyAgent.app` check, deterministic detach/cleanup, and
  artifact upload with missing-file failure.
- Run `pnpm exec vitest run tests/releaseWorkflow.test.ts`.
- For acceptance, require a successful GitHub `macos-14` run and inspect the
  uploaded artifact metadata. Windows static checks are not macOS evidence.

## 7. Wrong vs Correct

Wrong: let a feature-branch build enter the signed release steps with empty
Apple secrets, or silently label an unsigned DMG as a release package.

Correct: route the feature branch to an explicitly unsigned, verified workflow
artifact while keeping the `v*` path signed, notarized, Gatekeeper-checked, and
fail-closed.
