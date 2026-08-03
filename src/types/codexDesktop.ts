/**
 * Renderer contract for the Codex desktop installer.
 *
 * These shapes mirror the Rust IPC DTOs. They intentionally omit installer
 * paths, artifact URLs, hashes, package identities supplied by a mirror, and
 * any install-scope input.
 */

export type DesktopPlatform = "windows" | "macos";

export type CpuArchitecture =
  | "x86_64"
  | "aarch64"
  | "x86_64_unsupported_mac"
  | "unsupported";

export type PlatformVersion =
  | {
      kind: "windows_msix";
      major: number;
      minor: number;
      build: number;
      revision: number;
    }
  | {
      kind: "mac_bundle";
      bundleVersion: string;
    };

export interface InstalledApplication {
  stableIdentity: string;
  displayName: string | null;
  displayVersion: string | null;
  platformVersion: PlatformVersion;
  architecture: CpuArchitecture;
}

export interface InstalledApplicationSummary {
  stableIdentity: string;
  displayVersion: string | null;
  platformVersion: PlatformVersion;
  architecture: CpuArchitecture;
}

export type UnsupportedReason = "platform" | "architecture" | "os_version";

/**
 * Fixed, renderer-safe runtime summary for a possible Codex Desktop restart.
 * It deliberately carries no process identifier, executable path, or launch
 * command; all process identity decisions remain in the backend.
 */
export type CodexDesktopRuntimeStatus =
  | { state: "not_installed" }
  | { state: "not_running" }
  | { state: "running" }
  | {
      state: "ambiguous";
      reason: "installations" | "instances" | "identity_verification";
    }
  | {
      state: "unsupported";
      reason: UnsupportedReason;
    };

export type CodexDesktopRestartUnavailableReason =
  | "not_installed"
  | "not_running"
  | "installations_ambiguous"
  | "instances_ambiguous"
  | "identity_verification"
  | "unsupported";

export type CodexDesktopRestartPhase =
  | "detect"
  | "quit"
  | "force_quit"
  | "launch"
  | "verify";

/** The opaque force token is returned only to the trusted backend verbatim. */
export type CodexDesktopRestartOutcome =
  | { state: "restarted" }
  | { state: "force_confirmation_required"; token: string }
  | { state: "unavailable"; reason: CodexDesktopRestartUnavailableReason }
  | { state: "cancelled" }
  | {
      state: "failed";
      phase: CodexDesktopRestartPhase;
      error: InstallerErrorDto;
    };

export type InstallerErrorCode =
  | "PLATFORM_UNSUPPORTED"
  | "OS_VERSION_UNSUPPORTED"
  | "ARCHITECTURE_UNSUPPORTED"
  | "SOURCE_UNAVAILABLE"
  | "RELEASE_METADATA_INVALID"
  | "RELEASE_NOT_AVAILABLE"
  | "METADATA_CHANGED"
  | "REDIRECT_REJECTED"
  | "DOWNLOAD_FAILED"
  | "DOWNLOAD_TIMEOUT"
  | "DOWNLOAD_CANCELLED"
  | "INSUFFICIENT_DISK_SPACE"
  | "CHECKSUM_MISSING"
  | "CHECKSUM_MISMATCH"
  | "PACKAGE_PARSE_FAILED"
  | "PACKAGE_IDENTITY_MISMATCH"
  | "PACKAGE_ARCHITECTURE_MISMATCH"
  | "PACKAGE_SIGNATURE_INVALID"
  | "WINDOWS_UAC_CANCELLED"
  | "WINDOWS_ELEVATION_FAILED"
  | "WINDOWS_PACKAGE_IN_USE"
  | "WINDOWS_DEPLOYMENT_BLOCKED"
  | "WINDOWS_DEPENDENCY_MISSING"
  | "WINDOWS_ALL_USERS_UNSUPPORTED"
  | "WINDOWS_DEPLOYMENT_FAILED"
  | "MAC_DMG_MOUNT_FAILED"
  | "MAC_APP_NOT_FOUND"
  | "MAC_BUNDLE_ID_MISMATCH"
  | "MAC_TEAM_ID_MISMATCH"
  | "MAC_GATEKEEPER_REJECTED"
  | "MAC_APP_RUNNING"
  | "MAC_MULTIPLE_INSTALLATIONS"
  | "MAC_TARGET_PATH_CONFLICT"
  | "MAC_COPY_FAILED"
  | "MAC_DMG_DETACH_FAILED"
  | "INSTALLATION_VERIFY_FAILED"
  | "LAUNCH_FAILED"
  | "JOB_ALREADY_RUNNING"
  | "JOB_NOT_FOUND"
  | "INTERNAL_ERROR";

export type SuggestedAction =
  | "retry"
  | "refresh"
  | "close_target_app_and_retry"
  | "contact_administrator"
  | "free_disk_space"
  | "resolve_path_conflict"
  | "open_logs"
  | "none";

export interface InstallerDiagnosticDetails {
  endpointKind: string | null;
  attempt: number | null;
  maxAttempts: number | null;
  httpStatus: number | null;
  platformErrorCode: string | null;
  redactedMessage: string | null;
  context: Record<string, string>;
}

export interface InstallerErrorDto {
  code: InstallerErrorCode;
  stage: JobStage | null;
  messageKey: string;
  retryable: boolean;
  suggestedAction: SuggestedAction;
  details: InstallerDiagnosticDetails;
}

export type LocalInstallStatus =
  | {
      state: "not_installed";
      platform: DesktopPlatform;
      architecture: CpuArchitecture;
    }
  | {
      state: "installed";
      application: InstalledApplication;
    }
  | {
      state: "unsupported";
      reason: UnsupportedReason;
    }
  | {
      state: "ambiguous";
      candidates: InstalledApplicationSummary[];
      error: InstallerErrorDto;
    };

export interface RemoteReleaseStatus {
  releaseId: string;
  displayVersion: string;
  platformVersion: PlatformVersion;
  expectedSize: number;
  checkedAt: string;
}

export type InstallerWarningCode =
  | "temp_cleanup_failed"
  | "mac_dmg_detach_warning"
  | "log_write_failed"
  | "event_emit_failed"
  | "remote_check_failed_local_available";

export interface InstallResult {
  installed: InstalledApplicationSummary;
  warnings: InstallerWarningCode[];
}

export type JobStage =
  | "checking"
  | "preflight"
  | "downloading"
  | "verifying_download"
  | "installing"
  | "verifying_installation"
  | "succeeded"
  | "failed"
  | "cancelled";

export type ProgressPhase = "download" | "verification" | "installation";

export interface JobProgress {
  phase: ProgressPhase;
  completedBytes: number | null;
  totalBytes: number | null;
  percent: number | null;
}

export interface JobSnapshot {
  jobId: string;
  sequence: number;
  stage: JobStage;
  release: RemoteReleaseStatus;
  startedAt: string;
  updatedAt: string;
  progress: JobProgress | null;
  cancellable: boolean;
  result: InstallResult | null;
  error: InstallerErrorDto | null;
}

export type InstallerViewState =
  | "hidden"
  | "checking"
  | "unsupported_architecture"
  | "ambiguous"
  | "ready_install"
  | "ready_update"
  | "ready_launch"
  | "local_newer"
  | "remote_unavailable"
  | "remote_unavailable_installed"
  | "job_checking"
  | "job_preflight"
  | "job_downloading"
  | "job_verifying_download"
  | "job_installing"
  | "job_verifying_installation"
  | "succeeded"
  | "failed"
  | "cancelled";

export type InstallerPrimaryAction =
  | "install"
  | "update"
  | "launch"
  | "retry"
  | "refresh"
  | null;

export function isTerminalJobStage(stage: JobStage): boolean {
  return stage === "succeeded" || stage === "failed" || stage === "cancelled";
}

export function isInstalledLocalStatus(
  status: LocalInstallStatus | undefined,
): status is Extract<LocalInstallStatus, { state: "installed" }> {
  return status?.state === "installed";
}

export function displayPlatformVersion(version: PlatformVersion): string {
  if (version.kind === "windows_msix") {
    return [version.major, version.minor, version.build, version.revision].join(
      ".",
    );
  }

  return version.bundleVersion;
}

/**
 * Presentation-only comparison of canonical Rust DTOs. The backend remains
 * authoritative for source validation, compatibility, start-install checks,
 * and post-install verification; this only chooses the visible action.
 */
export function comparePlatformVersions(
  local: PlatformVersion,
  remote: PlatformVersion,
): -1 | 0 | 1 | null {
  if (local.kind !== remote.kind) {
    return null;
  }

  if (local.kind === "windows_msix" && remote.kind === "windows_msix") {
    const localParts = [local.major, local.minor, local.build, local.revision];
    const remoteParts = [
      remote.major,
      remote.minor,
      remote.build,
      remote.revision,
    ];

    for (let index = 0; index < localParts.length; index += 1) {
      if (localParts[index] < remoteParts[index]) return -1;
      if (localParts[index] > remoteParts[index]) return 1;
    }
    return 0;
  }

  if (local.kind === "mac_bundle" && remote.kind === "mac_bundle") {
    const localParts = local.bundleVersion.split(".");
    const remoteParts = remote.bundleVersion.split(".");
    const componentCount = Math.max(localParts.length, remoteParts.length);

    for (let index = 0; index < componentCount; index += 1) {
      const comparison = compareUnsignedVersionComponent(
        localParts[index] ?? "0",
        remoteParts[index] ?? "0",
      );
      if (comparison !== 0) return comparison;
    }
    return 0;
  }

  return null;
}

function compareUnsignedVersionComponent(
  left: string,
  right: string,
): -1 | 0 | 1 {
  const normalizedLeft = left.replace(/^0+(?=\d)/, "");
  const normalizedRight = right.replace(/^0+(?=\d)/, "");
  if (normalizedLeft.length < normalizedRight.length) return -1;
  if (normalizedLeft.length > normalizedRight.length) return 1;
  if (normalizedLeft < normalizedRight) return -1;
  if (normalizedLeft > normalizedRight) return 1;
  return 0;
}
