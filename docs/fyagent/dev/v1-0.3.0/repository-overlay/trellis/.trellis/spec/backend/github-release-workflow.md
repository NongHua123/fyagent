# GitHub CI and Release Workflow Contract

## Authority

GitHub Actions is the final multi-platform merge and formal-release authority. Local `mise run check` covers only the current host. Local bundles are not formal assets.

## Required CI

Required workflows listen to:

```yaml
pull_request: main
push: main
merge_group: checks_requested
workflow_dispatch:
```

A stable aggregate job such as `CI / Required` evaluates every required dependency result. `failure`, `cancelled`, and unexpected `skipped` are failures. Required workflows are not entirely skipped by path filters.

The exact workflow/job topology may change to fit the merged repository, provided the contract is preserved. CI consumes Node, pnpm, and Rust standard version files and does not install mise.

## Runners

Required CI and Release jobs use explicit OS labels, never `*-latest`. Current target policy:

| Target | Explicit runner/build baseline |
|---|---|
| Windows x64 | `windows-2022` |
| Windows ARM64 | `windows-11-arm` |
| macOS | `macos-15` |
| Linux x64 | explicit newer x64 host + same-architecture Ubuntu 22.04 container |
| Linux ARM64 | explicit newer ARM64 host + same-architecture Ubuntu 22.04 container |

Linux Release uses a reviewed container digest, no QEMU, and no cross-architecture build. The older user space preserves the intended glibc compatibility baseline.

## Action and permission security

- Every `uses:` reference is a full immutable commit SHA with a reviewed version comment.
- Default workflow permissions are `contents: read`.
- Only the final publish job gets `contents: write`.
- Only provenance jobs get `id-token: write` and `attestations: write`.
- PR jobs do not receive production signing secrets.
- Action updates are reviewed PRs and never auto-merged.

## Toolchain contract

The jobs install and then strictly verify:

```text
Node.js 24.19.0
pnpm 10.12.3
Rust 1.97.1 + rustfmt + clippy
```

Values come from `.node-version`, `package.json#packageManager`, and `rust-toolchain.toml`, not duplicated YAML constants. Runtime versions are checked on every relevant platform. Workflow static checks reject rolling channels, duplicate declarations, `*-latest`, retired cross targets, and unpinned Actions.

## Release eligibility and modes

Formal tags are `vX.Y.Z` and must match the product version contract. The source SHA is an ancestor of protected `main`, has successful Required CI, and is identical across all platform jobs.

Manual preflight supports:

- `unsigned`: five target groups, no production signing secrets, workflow artifacts only;
- `signed`: protected-main SHA, protected environment approval, Windows signing and macOS signing/notarization, workflow artifacts only.

Formal tag publication is fail-closed. Publish write permission is granted only after every platform, signing, structure, version, and asset check succeeds.

## Asset contract

Exactly ten installer assets:

```text
macOS: DMG, ZIP
Windows: x64 MSI, ARM64 MSI
Linux x64: AppImage, DEB, RPM
Linux ARM64: AppImage, DEB, RPM
```

A machine-readable manifest records filename, SHA-256, size, platform, architecture, product version, tag, source SHA, toolchain versions, and runner image. Missing, duplicate, or unexpected assets block publication.

Where platform/plan capabilities permit, final assets receive GitHub artifact attestations. When attestations are unavailable, the digest manifest remains mandatory and the capability gap is recorded rather than silently omitted.

## Tests Required

- workflow syntax and static-policy tests;
- aggregate-gate tests for success/failure/cancel/skip combinations;
- multi-platform toolchain runtime checks;
- unsigned full-matrix dry run;
- signed protected-environment preflight before production use;
- exact asset-set and manifest validation;
- Windows signature/MSI checks and macOS signature/notarization checks;
- Linux architecture and glibc-baseline evidence.
