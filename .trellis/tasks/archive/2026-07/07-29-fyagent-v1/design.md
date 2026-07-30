# FyAgent V1 技术设计

## Architecture Boundary

FyAgent V1 在不改变现有 Provider、Proxy、数据库、MCP、Skill、Prompt、Usage 和
用户数据目录的前提下，新增一个独立的桌面应用安装领域。依赖方向固定为：

```text
Tauri command → CodexDesktopService → codex_desktop domain/platform
React component → hook → Query/API → Tauri command/event
```

`commands` 只进行 DTO 输入和状态获取，`CodexDesktopService` 拥有单任务、缓存、
取消、流程编排和事件；领域模块拥有 source、下载、校验、版本与平台适配。React
只合并后端快照，不能重建状态机或信任前端传入 URL、路径、hash、scope。

## Data and Trust Flow

```text
fixed agentsmirror endpoints
  → raw checksums + raw manifest (hash before JSON parse)
  → restricted ReleaseDescriptor + canonical release_id
  → force revalidation at start_install
  → fixed platform short link + manual HTTPS redirects
  → fixed temp path + size/hash validation
  → platform package identity/architecture/signature validation
  → reopen fixed job artifact + exact descriptor size/hash revalidation
  → current-user installation
  → OS re-detection + complete JobSnapshot event
```

The descriptor excludes remote URL authority. The source selects a URL only from a compile-time
endpoint enum (`manifest`, `checksums`, `win-x64`, `win-arm64`, `mac-arm64`). Dynamic manifest
URLs, deltas, upstream metadata, query strings and unsupported architectures are ignored.

Current schema evidence is version 5. Parsing is version-gated, bounded and fail closed; unknown
fields inside a supported schema can be ignored, but an unsupported major schema, missing current
platform field, malformed hash/version/size or missing matching checksum fails with a stable code.

## Core Contracts

`codex_desktop/types.rs` contains platform/architecture, platform-specific versions,
`LocalInstallStatus`, `RemoteReleaseStatus`, `JobSnapshot`, result/warning DTOs, and a canonical
`release_id`. Serde casing is explicitly tested against the TypeScript contract fixture.

`codex_desktop/error.rs` maps every error to a stable code, user message key, retryability,
suggested action and bounded/redacted diagnostics. Diagnostic formatters redact query/fragment,
userinfo, authorization, home paths and oversized command output.

`codex_desktop/platform/mod.rs` exposes a narrow `CodexDesktopPlatform` trait. Only a
`VerifiedPackage` can reach `install_current_user`; it cannot be forged from an arbitrary downloaded
path. It retains the locked descriptor and downloader-owned job-artifact capability; immediately
before Windows deployment or macOS DMG mount it reopens that fixed regular/non-link artifact and
rechecks exact size/SHA-256, so a package verified earlier cannot be consumed after replacement.
The normal trait does not contain all-users installation. Linux/unsupported implementations compile
and return an explicit unsupported status.

`CodexDesktopService` holds:

- a five-minute in-memory descriptor cache keyed by platform/architecture;
- one mutex/atomic protected active Job controller;
- cancellation token and sequence-monotonic snapshot publisher;
- injectable source, HTTP builder/downloader, platform adapter, clock, filesystem/free-space probe,
  event sink and command runner for deterministic tests.

The service always re-fetches metadata for `start_install(expected_release_id)`. A mismatch returns
`METADATA_CHANGED` before any download. Long I/O never holds the job mutex. Terminal jobs cannot
update a newer job, and event delivery failure does not change the authoritative result.

## Platform Designs

### Windows

Windows code is entirely under `cfg(target_os = "windows")`. The adapter parses a bounded root
`AppxManifest.xml`, requires exact `OpenAI.Codex`, current-architecture MSIX version, one launchable
Application ID and a fixture-derived exact Publisher. It uses a fakeable PackageManager facade for
local inventory, `file://` current-user deployment, HRESULT mapping, post-install inventory and
AUMID activation. It never uses PowerShell, winget, direct WindowsApps executable launch,
ForceApplicationShutdown or a fallback x64 package on ARM64.

All-users provisioning is separate from ordinary Tauri IPC: a strict pre-runtime headless command
creates/reads a nonce-bound, expiry-limited job description under a controlled temp root, revalidates
the file/hash/manifest/identity/publisher/architecture after UAC, then stages/provisions only if the
system allows it. It is testable but does not determine V1 release acceptance.

### macOS

macOS code is entirely under `cfg(target_os = "macos")` and uses an injectable command runner.
It scans only top-level `/Applications/*.app` and `~/Applications/*.app`; Bundle ID is the identity
key, so Classic and Beta are never overwritten. Scanning first uses a tolerant Bundle-ID probe:
malformed unrelated candidates are skipped, while an exact Stable candidate is re-read and remains
fully fail closed. Exactly one Stable bundle is required for update or launch. A DMG mount guard
parses `hdiutil -plist`, requires exactly one top-level `.app`, validates
the app's `Info.plist`, arm64 architecture, `codesign`, Team ID `2DC432GLL2`, and `spctl` before
and after copy.

New installations preserve the DMG basename and try `/Applications` before `~/Applications` only
on an actual permission error. Updates preserve the existing canonical Stable path. `ditto` copies
to a same-volume randomized staging sibling; an update renames the verified existing target to a
short-lived backup, swaps staging atomically and restores on failure. The adapter never uses sudo,
administrator privileges, `xattr` quarantine removal, resigning, `open -a`, name-based process
matching or forceful application termination.

## Frontend Design

The UI is isolated in TypeScript types, a thin `invoke` API, Query keys/mutations, a single
`useCodexDesktopInstaller` hook and `CodexDesktopInstallerCard`. On mount the hook subscribes to
`codex-desktop-installer://job-updated`, then obtains `get_job`, retaining the newest
`jobId + sequence`. It makes local and remote checks in parallel, preserves local state when remote
check fails, and invalidates terminal state exactly once.

The card renders the state table from the V1 specification using existing primitives. It has no
source selector, URL/path/hash controls, scope selector, reinstall/downgrade affordance or automatic
Provider mutation. It does not render on Linux, explicitly reports Intel macOS unsupported, keeps
the primary launch action available when only the remote request fails, and shows the success toast
once per job. All visible installer text exists in `zh.json` and `en.json`; other locales use the
existing English fallback.

## Integration and Compatibility

`AppState` owns one service instance. Shared registration (`lib.rs`, `store.rs`, `commands/mod.rs`,
`services/mod.rs`, Cargo files, updater/config, and root UI composition) is changed only after the
Core API contracts have compiled. Existing global proxy behavior is reused through a narrow scoped
HTTP client builder; the current singleton's redirect/timeout behavior is not changed.

Codex CLI lifecycle actions are centrally guarded by a capability helper or equivalent single
decision point: direct back-end write requests fail while read-only detection survives. Branding is
restricted to user-visible names/links and updater removal; identifiers, data directory, deep-link,
crate/npm names and legal notices remain unchanged.

Updater removal also covers the existing database-version recovery branch and release artifact
workflow. A database that is newer than the executable remains a safe blocking state; instead of
checking or installing an upstream release, the UI provides a translated, non-network recovery
message directing the internal user to the controlled FyAgent distribution/support path. This keeps
data compatibility protection without preserving an upstream update channel or a dead invoke.

## Rollback and Operational Shape

The work is divided into atomic commits and child tasks. A failing platform commit can be reverted
without rewriting shared history; unsupported UI state must remain accurate until that platform is
repaired. Runtime temp data is job-scoped: successful/cancelled files are removed promptly; stale
canonical child directories older than 24 hours are cleaned on startup without following symlinks.
No job, remote descriptor or installer package is persisted to the application database/settings.

Automated checks use fixtures, fake platform/deployment/command facades and mock HTTP only. The
manual test matrix is explicitly a later human-owned stage, not a test script or an agent claim.
The integration evidence additionally records Windows ARM64 compilation on a suitable target or
runner; a successful x64 Windows build is not treated as ARM64 proof.
