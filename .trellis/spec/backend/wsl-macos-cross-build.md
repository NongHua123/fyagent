# WSL macOS Universal DMG Cross-Build Contract

## 1. Scope / Trigger

This contract applies to the repository-owned workflow under
`scripts/macos-cross/`. It produces one experimental macOS Universal DMG from
WSL2 Ubuntu for internal testing. It is separate from
`.github/workflows/release.yml` and must not weaken the native signed and
notarized release path.

## 2. Signatures

The supported mise entry point delegates to the repository-owned script and
produces these local outputs:

```text
mise run build:cross-macos:universal
./scripts/macos-cross/build-universal-dmg.sh [--accept-risk]

dist-bundle/macos/FyAgent-<version>-macOS-universal-adhoc-unnotarized-experimental.dmg
dist-bundle/macos/FyAgent-<version>-macOS-universal-adhoc-unnotarized-experimental.dmg.sha256
dist-bundle/macos/FyAgent-<version>-macOS-universal-adhoc-unnotarized-experimental.dmg.manifest.json
```

The build target is always `universal-apple-darwin`, containing exactly
`arm64` and `x86_64`. There is no public single-architecture or ZIP mode.

## 3. Contracts

- The supported host is WSL2 x86_64 with Ubuntu 22.04 or 24.04 and an ext4
  project filesystem. Windows-mounted project paths fail before provisioning.
- The workflow requires user-installed global mise `>= 2026.8.0`, resolves its
  absolute Linux path before sanitizing PATH, and uses repository
  `mise.toml`/`mise.lock` versions for Node, pnpm, Python, Rust, and both macOS
  Rust targets. It never downloads a private mise or selects executables from
  `/mnt`.
- The SDK URL/hash, OSXCross commit, libdmg-hfsplus commit, rcodesign version,
  and rcodesign archive hash are immutable repository constants. Moving refs
  such as `master`, `main`, or `latest` are forbidden.
- Built-tool cache keys include the supported Ubuntu version and host
  architecture in addition to SDK hash, source commit, build flavor, and
  deployment target inputs. A cache marker mismatch is a miss, never a reason
  to trust an old executable.
- The first use of a new fixed-input set requires an explicit third-party SDK,
  Apple license, GPL-3.0 tool, and experimental DMG acknowledgement. The
  purpose-built mise task retains the interactive prompt and does not
  auto-install missing tools before the wrapper runs; in a non-interactive
  environment, rerun it as `mise run build:cross-macos:universal --accept-risk`.
- Manual app assembly supports only the current main binary, ICNS icon,
  deep-link schemes, and custom Info.plist overrides. Any configured resources,
  sidecars, file associations, frameworks, files, entitlements, signing
  identity, DMG customization, or explicit extra binary target fails closed.
- Dependency installation runs inside the inherited mise environment as the
  equivalent of `CI=true mise exec -- pnpm install --frozen-lockfile`. The CI
  environment is required so the mise-managed pnpm store can replace an
  incompatible existing `node_modules` directory without an interactive
  confirmation prompt.
- The Universal Mach-O must contain exactly both slices, target macOS 12.0,
  and contain no Linux/WSL load paths before app assembly.
- The fully assembled app is ad-hoc signed before it enters the DMG. The DMG is
  ad-hoc signed before checksum and manifest generation. Signature inspection
  must report `CodeSignatureFlags(ADHOC)` and no CMS identity. Neither step
  reads Apple signing, notarization, or release credentials.
- The pinned libdmg build is limited to the unencrypted `dmg-bin` target and
  records `fileVault=disabled`. It must not link host `libcrypto`; encrypted
  FileVault images are outside this workflow.
- GNU `file` validates UDIF through the repository-owned `udif.magic` trailer
  rule because its default Linux magic database may identify the compressed
  payload instead of the outer image. The independent final-512-byte `koly`
  check remains mandatory.
- Final publication happens only after app, Mach-O, UDIF `koly`, DMG signature,
  checksum, and manifest checks pass. The manifest is checked against the
  actual DMG before and after publication. Failure must not publish a new DMG.
- Git worktree cleanliness is deliberately unchecked. The manifest may record
  HEAD as context but must not claim the artifact is reproducible from it.
- The manifest states `macosNativeValidation: pending`; WSL evidence must never
  be described as native mount, install, launch, codesign, Gatekeeper, or
  notarization acceptance.

## 4. Validation & Error Matrix

| Condition                                                   | Required result                              |
| ----------------------------------------------------------- | -------------------------------------------- |
| Unsupported host, distro, architecture, or filesystem       | Fail before download/build                   |
| Global mise is absent, too old, or resolves under `/mnt`    | Fail before host mutation or provisioning    |
| First non-interactive run lacks `--accept-risk`             | Fail before SDK/tool download                |
| Any fixed checksum or git commit differs                    | Fail and retain no promoted input            |
| Runtime differs from global `mise which` or is under `/mnt` | Fail before dependency install               |
| Unsupported Tauri bundle content appears                    | Fail before compiling/package assembly       |
| pnpm store metadata differs from the modules directory      | Reinstall from frozen lock without prompting |
| One Mach-O slice is missing or a Linux load path appears    | Fail before app assembly                     |
| App or DMG signing/verification fails                       | Fail before final checksum/publication       |
| DMG lacks `koly` or checksum/manifest differs               | Fail without replacing final DMG             |
| Static checks pass but no real Mac was used                 | Mark native acceptance pending               |

## 5. Good / Base / Bad Cases

- Good: global mise is installed but project versions may be absent; a first
  run accepts the fixed-source risk, runs `mise install`, provisions pinned
  cross tools, builds both slices, and publishes exactly the DMG, checksum, and
  manifest. The next run logs cross-tool cache hits and rebuilds/revalidates the
  application.
- Base: a valid cache already exists for the exact host and fixed inputs; reuse
  the tools while repeating all project, binary, signing, UDIF, checksum, and
  manifest checks.
- Bad: reuse a cache from another host, continue after one slice fails, accept
  a non-ad-hoc signature, or publish a manifest whose size/hash does not match
  the final DMG.

## 6. Tests Required

- Run Bash syntax checks and ShellCheck for every workflow script.
- Run the Python unit suite for configuration, plist, architecture,
  contamination, and UDIF validation behavior.
- Run `tests/macosCrossWorkflow.test.ts` together with the existing release and
  version consistency tests.
- Assert the workflow requires global mise, never downloads mise, and resolves
  every managed command to the same path as `mise which`.
- Complete one real first-run WSL build and a second cache-reuse build. Verify
  the published SHA256 and manifest after both runs.
- Perform native macOS mounting, installation, launch, deep-link, codesign, and
  Gatekeeper checks later; keep their status pending until actual evidence
  exists.

## 7. Wrong vs Correct

Wrong: publish a single-architecture, unsigned, ambiguously named DMG from WSL
or route this workflow into the GitHub release job.

Correct: produce one explicitly ad-hoc, unnotarized, experimental Universal
DMG, validate everything observable from WSL, and preserve the native macOS
release and acceptance boundary.
