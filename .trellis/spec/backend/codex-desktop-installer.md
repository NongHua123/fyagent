# Codex Desktop Installer Contract

## 1. Scope / Trigger

The Codex desktop installer spans Rust domain/source/platform code, seven Tauri
commands, TypeScript query/hook/card consumers, and platform-specific
verification. The frozen product requirements are in docs/fyagent/dev/v1/;
this specification preserves the executable cross-layer boundary so later work
does not reintroduce user-controlled installer inputs or drift the wire
format.

## 2. Signatures

The ordinary Tauri command surface is exactly:

    codex_desktop_get_local_status()
    codex_desktop_check_latest(force: bool)
    codex_desktop_get_job()
    codex_desktop_start_install(request: StartInstallRequest)
    codex_desktop_cancel_install(job_id: string)
    codex_desktop_launch()
    codex_desktop_open_log_directory()

StartInstallRequest serializes as:

    { "expectedReleaseId": "v1:<64 lowercase hex characters>" }

No ordinary command accepts a URL, path, hash, identity, installer scope, or
validation-bypass flag. The hidden Windows all-users experiment is a
pre-runtime headless boundary, never one of these commands.

## 3. Contracts

### Source to service

- ReleaseSource may request only the fixed AgentsMirror manifest/checksum and
  platform short-link endpoints represented by TrustedDownloadEndpoint.
- Metadata URLs, redirect targets, delta URLs, and remote filesystem names
  never become a caller-controlled download target.
- The fixed manifest/checksum endpoints use the same manual redirect policy as
  package requests: at most five hops, HTTPS only, no user information, and
  no persisted final URL. This permits the current fixed endpoint's R2
  redirect without turning redirect metadata into a renderer capability.
- The release descriptor is valid only after manifest checksum, derived
  checksum, checksum-file, architecture, size, and platform-version checks
  agree.
- Windows derives the MSIX name from its validated package moniker. macOS
  derives the single safe .dmg name from the validated branch/derived checksum
  records; a missing or ambiguous match fails closed.

### Rust to renderer

- Rust serializes camelCase DTO fields and snake_case tagged enum values where
  declared by serde.
- JobSnapshot is a complete authoritative snapshot. The renderer merges events
  by jobId and monotonic sequence; it does not implement installer state
  transitions itself.
- The canonical fixture tests/fixtures/codexDesktopDtoContract.v1.json is
  produced-equivalent to Rust DTO serialization and parsed by the TypeScript
  contract test.

### Job lifecycle

- At most one job may occupy the installer slot.
- A cancellation request keeps that slot until the worker has stopped
  cancellable work and cleaned its temporary files; only then may the snapshot
  become cancelled.
- A settings-triggered application restart must call
  `CodexDesktopService::claim_restart()` before saving state, cleanup, or
  re-exec. `claim_restart()` and `JobStore::try_start()` share one mutex: only
  an empty or terminal slot may claim restart; a cancellable, cancellation-
  pending, installing, or post-install-verifying job returns
  `JOB_ALREADY_RUNNING` without being cancelled or replaced. A successful
  claim rejects later starts until process re-exec clears the in-memory state.
- The renderer's default Tauri capability must not grant
  `process:allow-restart`. Renderer-initiated restarts use the controlled
  `restart_app` command above, so no frontend path can bypass the restart
  claim while an installer job owns the slot. The narrower
  `process:allow-exit` may remain for established explicit quit paths, which
  must continue through the application exit guard; never replace it with
  `process:default`.
- Before an install, the service force-refreshes metadata and rejects a changed
  releaseId with METADATA_CHANGED; the renderer must refresh and require a
  separate new Install/Update action.
- After that refresh and before preflight/download, the service re-detects the
  trusted local Stable application. If its platform version is equal to or
  newer than the descriptor, it launches that verified application and finishes
  the job through the narrow launch-only success path. It must create no temp
  directory and perform no disk probe, download, package validation, or install.
- A terminal checksum mismatch first deletes the fixed local artifact, then
  force-refreshes metadata exactly once for classification: a changed releaseId
  becomes METADATA_CHANGED; an unchanged releaseId remains CHECKSUM_MISMATCH.
- `VerifiedPackage` retains the locked descriptor, not only a previously
  verified path. Immediately before each platform consumption, it must reopen
  the fixed artifact under its canonical UUID job directory, reject a non-
  regular/link/reparse/path-drift artifact, and recheck exact size and SHA-256
  against that descriptor. Windows deploy and macOS DMG attach must not run
  after this check fails; macOS validates the mounted Stable bundle against the
  same descriptor's exact platform version.
- macOS standard-directory scanning uses a tolerant identifier probe before
  Stable-only validation. A malformed, non-file, missing-identity, or
  parse-rejected unrelated bundle is skipped;
  only an exact `com.openai.codex` probe enters the strict version, executable,
  architecture, Team, codesign, and Gatekeeper checks. Canonical-path escape,
  directory-enumeration failure, or a known Stable candidate's strict failure
  remains fail closed.

### Experimental all-users boundary

- All-users provisioning is a Windows-only pre-runtime headless mode; it is
  never added to the ordinary renderer IPC surface.
- The elevated child rebuilds the fixed UUID/job-file capability path and asks
  an injected `AllUsersJobControlReader` for at most 16 KiB of JSON. Generic
  code must not `metadata`, `canonicalize`, or reopen that parent-owned path.
- The native reader opens the control file once with
  `FILE_FLAG_OPEN_REPARSE_POINT`, rejects a reparse leaf, verifies the final
  handle path is the expected fixed local drive and a regular file, then reads
  through that same handle. It never uses `fs::read(expected_job_path)`.
- The child independently force-refreshes its release anchor and binds it to
  the job before deployment. It writes no parent-temp result file; only a
  protected ProgramData copy that is rehashed/revalidated may reach Stage and
  Provision.

## 4. Validation & Error Matrix

| Condition                                                                                              | Required result                                                                                                                              |
| ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Remote metadata has a changed release ID                                                               | METADATA_CHANGED, suggested action refresh; do not install the new release implicitly.                                                       |
| Locked metadata's artifact checksum mismatches and the refresh changes release ID                      | METADATA_CHANGED; delete the artifact and require an explicit refreshed action.                                                              |
| Trusted local Stable version is equal to or newer than the descriptor                                  | Launch only; no preflight, download, temporary directory, package validation, or install.                                                    |
| Another job is active or cancellation cleanup is pending                                               | JOB_ALREADY_RUNNING; retain the single-job slot.                                                                                             |
| Settings restart races with a start request                                                            | Exactly one may claim the same mutex; a running/cancellation-pending job blocks restart, and a successful restart claim blocks later starts. |
| Hash sources disagree or an artifact name is unsafe/ambiguous                                          | CHECKSUM_MISMATCH, CHECKSUM_MISSING, or RELEASE_METADATA_INVALID; never guess an artifact.                                                   |
| Metadata or download redirect leaves HTTPS/allowlist policy                                            | REDIRECT_REJECTED.                                                                                                                           |
| Artifact changes after package verification but before a platform consumes it                          | CHECKSUM_MISMATCH or a stable artifact-validation error; do not call deploy or attach.                                                       |
| Download is cancelled before installation                                                              | Worker cleans temp data, then publishes cancelled.                                                                                           |
| Platform verification, signature, identity, architecture, or post-check fails                          | Stable platform/package error; do not launch or downgrade.                                                                                   |
| Ordinary renderer tries to provide scope/URL/path/extra request field                                  | DTO deserialization or validation rejects it.                                                                                                |
| Elevated all-users job control is empty, oversized, reparse-backed, remote, or changes capability path | WINDOWS_ELEVATION_FAILED before the fresh anchor, validator, Stage, or Provision adapter runs.                                               |

Diagnostics may contain only the structured, redacted fields of
InstallerErrorDto; never pass raw credential-bearing URLs, paths, cookies, or
installer command lines to the renderer.

## 5. Good / Base / Bad Cases

### Good

The renderer observes release A, calls start_install with expectedReleaseId A,
and the service re-resolves A before it downloads through the fixed platform
endpoint. A complete snapshot event lets the renderer show progress.

### Base

Metadata is unavailable while a verified local app exists. The service reports
the remote failure while the renderer retains the separate local Launch action.

### Bad

    // Wrong: turns a metadata response into a download capability.
    invoke("codex_desktop_start_install", {
      request: { expectedReleaseId, url, path, scope: "all_users" },
    });

The command DTO must reject the extra fields, and the renderer must never offer
such controls.

## 6. Tests Required

- Rust: source checksum/artifact-name derivation, release-ID vectors, job
  transition/sequence/cancellation races, restart-claim-versus-start race,
  default-capability rejection of an uncoordinated renderer process restart
  while retaining only the guarded exit capability,
  replacement-after-verification regression paths that prove Windows deployment
  and macOS attach are not reached,
  macOS malformed-unrelated-bundle scan regressions alongside known-Stable
  fail-closed fixtures,
  service metadata-drift and checksum-reanchor behavior, direct same/newer
  local-version launch-only behavior, platform fixture/fake tests, and DTO
  fixture equality.
- TypeScript: import and parse tests/fixtures/codexDesktopDtoContract.v1.json;
  enumerate every frozen enum/tag branch and consume the complete snapshot.
- Integration: static audit that ordinary IPC has no all-users or custom-input
  surface, and each command remains registered exactly once.
- All-users: inject a bounded control reader; assert oversized JSON rejects
  before metadata/native adapters, generic code has no path reopen, and the
  Windows reader has a no-follow same-handle/final-path/fixed-drive audit.
- Platform acceptance: real Windows x64, Windows ARM64, Apple Silicon macOS,
  and mainland-network checks remain human-owned and are not replaced by these
  tests.

## 7. Wrong vs Correct

### Wrong

    // A fixed marketing filename breaks when the signed upstream DMG is renamed.
    let artifact_name = "Codex-mac-arm64.dmg";

### Correct

    // Select only the unique safe DMG whose checksum record agrees with the
    // validated macOS branch, then keep downloading through the fixed endpoint.
    let artifact_name = derive_macos_arm64_artifact_file_name(
        &manifest.derived.latest_checksums,
        artifact.sha256.as_deref(),
    )?;

The filename is data used to cross-check integrity metadata, not a path or
remote URL capability.

### Wrong

```rust
// Checking a parent-owned path and later reopening it leaves a TOCTOU window.
let metadata = std::fs::metadata(expected_job_path)?;
let bytes = std::fs::read(expected_job_path)?;
```

### Correct

```rust
// Native code verifies and consumes one no-follow handle; generic protocol
// code receives only the already-bounded byte vector.
let bytes = job_control_reader.read_job_control(expected_job_path, 16 * 1024)?;
```

Likewise, do not merely inspect the installer job before a delayed restart:
claim the shared job-store mutex first, so a new worker cannot start during the
response/re-exec window.
