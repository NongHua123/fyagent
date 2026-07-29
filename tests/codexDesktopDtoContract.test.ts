import { describe, expect, it } from "vitest";
import fixtureJson from "./fixtures/codexDesktopDtoContract.v1.json";
import type {
  CpuArchitecture,
  DesktopPlatform,
  InstallResult,
  InstallerErrorCode,
  InstallerWarningCode,
  JobSnapshot,
  JobStage,
  LocalInstallStatus,
  PlatformVersion,
  ProgressPhase,
  SuggestedAction,
  UnsupportedReason,
} from "@/types/codexDesktop";

const desktopPlatforms = [
  "windows",
  "macos",
] as const satisfies readonly DesktopPlatform[];
const cpuArchitectures = [
  "x86_64",
  "aarch64",
  "x86_64_unsupported_mac",
  "unsupported",
] as const satisfies readonly CpuArchitecture[];
const unsupportedReasons = [
  "platform",
  "architecture",
  "os_version",
] as const satisfies readonly UnsupportedReason[];
const installerWarningCodes = [
  "temp_cleanup_failed",
  "mac_dmg_detach_warning",
  "log_write_failed",
  "event_emit_failed",
  "remote_check_failed_local_available",
] as const satisfies readonly InstallerWarningCode[];
const jobStages = [
  "checking",
  "preflight",
  "downloading",
  "verifying_download",
  "installing",
  "verifying_installation",
  "succeeded",
  "failed",
  "cancelled",
] as const satisfies readonly JobStage[];
const progressPhases = [
  "download",
  "verification",
  "installation",
] as const satisfies readonly ProgressPhase[];
const installerErrorCodes = [
  "PLATFORM_UNSUPPORTED",
  "OS_VERSION_UNSUPPORTED",
  "ARCHITECTURE_UNSUPPORTED",
  "SOURCE_UNAVAILABLE",
  "RELEASE_METADATA_INVALID",
  "RELEASE_NOT_AVAILABLE",
  "METADATA_CHANGED",
  "REDIRECT_REJECTED",
  "DOWNLOAD_FAILED",
  "DOWNLOAD_TIMEOUT",
  "DOWNLOAD_CANCELLED",
  "INSUFFICIENT_DISK_SPACE",
  "CHECKSUM_MISSING",
  "CHECKSUM_MISMATCH",
  "PACKAGE_PARSE_FAILED",
  "PACKAGE_IDENTITY_MISMATCH",
  "PACKAGE_ARCHITECTURE_MISMATCH",
  "PACKAGE_SIGNATURE_INVALID",
  "WINDOWS_UAC_CANCELLED",
  "WINDOWS_ELEVATION_FAILED",
  "WINDOWS_PACKAGE_IN_USE",
  "WINDOWS_DEPLOYMENT_BLOCKED",
  "WINDOWS_DEPENDENCY_MISSING",
  "WINDOWS_ALL_USERS_UNSUPPORTED",
  "WINDOWS_DEPLOYMENT_FAILED",
  "MAC_DMG_MOUNT_FAILED",
  "MAC_APP_NOT_FOUND",
  "MAC_BUNDLE_ID_MISMATCH",
  "MAC_TEAM_ID_MISMATCH",
  "MAC_GATEKEEPER_REJECTED",
  "MAC_APP_RUNNING",
  "MAC_MULTIPLE_INSTALLATIONS",
  "MAC_TARGET_PATH_CONFLICT",
  "MAC_COPY_FAILED",
  "MAC_DMG_DETACH_FAILED",
  "INSTALLATION_VERIFY_FAILED",
  "LAUNCH_FAILED",
  "JOB_ALREADY_RUNNING",
  "JOB_NOT_FOUND",
  "INTERNAL_ERROR",
] as const satisfies readonly InstallerErrorCode[];
const suggestedActions = [
  "retry",
  "refresh",
  "close_target_app_and_retry",
  "contact_administrator",
  "free_disk_space",
  "resolve_path_conflict",
  "open_logs",
  "none",
] as const satisfies readonly SuggestedAction[];

type DtoContractFixture = {
  contractVersion: 1;
  desktopPlatforms: DesktopPlatform[];
  cpuArchitectures: CpuArchitecture[];
  platformVersions: PlatformVersion[];
  unsupportedReasons: UnsupportedReason[];
  localInstallStatuses: LocalInstallStatus[];
  installerWarningCodes: InstallerWarningCode[];
  jobStages: JobStage[];
  progressPhases: ProgressPhase[];
  installerErrorCodes: InstallerErrorCode[];
  suggestedActions: SuggestedAction[];
  startInstallRequest: { expectedReleaseId: string };
  installResult: InstallResult;
  jobSnapshot: JobSnapshot;
};

function asRecord(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(field + " must be an object");
  }
  return value as Record<string, unknown>;
}

function asArray(value: unknown, field: string): unknown[] {
  if (!Array.isArray(value)) {
    throw new Error(field + " must be an array");
  }
  return value;
}

function asString(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new Error(field + " must be a string");
  }
  return value;
}

function asBoolean(value: unknown, field: string): boolean {
  if (typeof value !== "boolean") {
    throw new Error(field + " must be a boolean");
  }
  return value;
}

function asNumber(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(field + " must be a finite number");
  }
  return value;
}

function asNullable(
  value: unknown,
  field: string,
  parser: (value: unknown, field: string) => void,
) {
  if (value !== null) {
    parser(value, field);
  }
}

function assertKnownString<T extends string>(
  value: unknown,
  field: string,
  values: readonly T[],
): void {
  const stringValue = asString(value, field);
  if (!values.includes(stringValue as T)) {
    throw new Error(field + " has an unknown value: " + stringValue);
  }
}

function assertExactEnumArray<T extends string>(
  value: unknown,
  field: string,
  expected: readonly T[],
): void {
  const actual = asArray(value, field);
  expect(actual).toEqual(expected);
}

function assertPlatformVersion(value: unknown, field: string): void {
  const record = asRecord(value, field);
  const kind = asString(record.kind, field + ".kind");
  if (kind === "windows_msix") {
    asNumber(record.major, field + ".major");
    asNumber(record.minor, field + ".minor");
    asNumber(record.build, field + ".build");
    asNumber(record.revision, field + ".revision");
    return;
  }
  if (kind === "mac_bundle") {
    asString(record.bundleVersion, field + ".bundleVersion");
    return;
  }
  throw new Error(field + ".kind is unknown: " + kind);
}

function assertInstalledSummary(value: unknown, field: string): void {
  const record = asRecord(value, field);
  asString(record.stableIdentity, field + ".stableIdentity");
  asNullable(record.displayVersion, field + ".displayVersion", asString);
  assertPlatformVersion(record.platformVersion, field + ".platformVersion");
  assertKnownString(
    record.architecture,
    field + ".architecture",
    cpuArchitectures,
  );
}

function assertInstalledApplication(value: unknown, field: string): void {
  const record = asRecord(value, field);
  asString(record.stableIdentity, field + ".stableIdentity");
  asNullable(record.displayName, field + ".displayName", asString);
  asNullable(record.displayVersion, field + ".displayVersion", asString);
  assertPlatformVersion(record.platformVersion, field + ".platformVersion");
  assertKnownString(
    record.architecture,
    field + ".architecture",
    cpuArchitectures,
  );
}

function assertInstallerError(value: unknown, field: string): void {
  const record = asRecord(value, field);
  assertKnownString(record.code, field + ".code", installerErrorCodes);
  asNullable(record.stage, field + ".stage", (stage, stageField) =>
    assertKnownString(stage, stageField, jobStages),
  );
  asString(record.messageKey, field + ".messageKey");
  asBoolean(record.retryable, field + ".retryable");
  assertKnownString(
    record.suggestedAction,
    field + ".suggestedAction",
    suggestedActions,
  );
  const details = asRecord(record.details, field + ".details");
  asNullable(details.endpointKind, field + ".details.endpointKind", asString);
  asNullable(details.attempt, field + ".details.attempt", asNumber);
  asNullable(details.maxAttempts, field + ".details.maxAttempts", asNumber);
  asNullable(details.httpStatus, field + ".details.httpStatus", asNumber);
  asNullable(
    details.platformErrorCode,
    field + ".details.platformErrorCode",
    asString,
  );
  asNullable(
    details.redactedMessage,
    field + ".details.redactedMessage",
    asString,
  );
  const context = asRecord(details.context, field + ".details.context");
  for (const [key, contextValue] of Object.entries(context)) {
    asString(contextValue, field + ".details.context." + key);
  }
}

function assertLocalInstallStatus(value: unknown, field: string): void {
  const record = asRecord(value, field);
  const state = asString(record.state, field + ".state");
  if (state === "not_installed") {
    assertKnownString(record.platform, field + ".platform", desktopPlatforms);
    assertKnownString(
      record.architecture,
      field + ".architecture",
      cpuArchitectures,
    );
    return;
  }
  if (state === "installed") {
    assertInstalledApplication(record.application, field + ".application");
    return;
  }
  if (state === "unsupported") {
    assertKnownString(record.reason, field + ".reason", unsupportedReasons);
    return;
  }
  if (state === "ambiguous") {
    asArray(record.candidates, field + ".candidates").forEach(
      (candidate, index) =>
        assertInstalledSummary(candidate, field + ".candidates[" + index + "]"),
    );
    assertInstallerError(record.error, field + ".error");
    return;
  }
  throw new Error(field + ".state is unknown: " + state);
}

function assertInstallResult(value: unknown, field: string): void {
  const record = asRecord(value, field);
  assertInstalledSummary(record.installed, field + ".installed");
  asArray(record.warnings, field + ".warnings").forEach((warning, index) =>
    assertKnownString(
      warning,
      field + ".warnings[" + index + "]",
      installerWarningCodes,
    ),
  );
}

function assertJobSnapshot(value: unknown, field: string): void {
  const record = asRecord(value, field);
  asString(record.jobId, field + ".jobId");
  asNumber(record.sequence, field + ".sequence");
  assertKnownString(record.stage, field + ".stage", jobStages);
  const release = asRecord(record.release, field + ".release");
  asString(release.releaseId, field + ".release.releaseId");
  asString(release.displayVersion, field + ".release.displayVersion");
  assertPlatformVersion(
    release.platformVersion,
    field + ".release.platformVersion",
  );
  asNumber(release.expectedSize, field + ".release.expectedSize");
  asString(release.checkedAt, field + ".release.checkedAt");
  asString(record.startedAt, field + ".startedAt");
  asString(record.updatedAt, field + ".updatedAt");
  if (record.progress !== null) {
    const progress = asRecord(record.progress, field + ".progress");
    assertKnownString(
      progress.phase,
      field + ".progress.phase",
      progressPhases,
    );
    asNullable(
      progress.completedBytes,
      field + ".progress.completedBytes",
      asNumber,
    );
    asNullable(progress.totalBytes, field + ".progress.totalBytes", asNumber);
    asNullable(progress.percent, field + ".progress.percent", asNumber);
  }
  asBoolean(record.cancellable, field + ".cancellable");
  asNullable(record.result, field + ".result", assertInstallResult);
  asNullable(record.error, field + ".error", assertInstallerError);
}

function assertContractFixture(
  value: unknown,
): asserts value is DtoContractFixture {
  const fixture = asRecord(value, "fixture");
  if (fixture.contractVersion !== 1) {
    throw new Error("fixture.contractVersion must be 1");
  }
  assertExactEnumArray(
    fixture.desktopPlatforms,
    "desktopPlatforms",
    desktopPlatforms,
  );
  assertExactEnumArray(
    fixture.cpuArchitectures,
    "cpuArchitectures",
    cpuArchitectures,
  );
  assertExactEnumArray(
    fixture.unsupportedReasons,
    "unsupportedReasons",
    unsupportedReasons,
  );
  assertExactEnumArray(
    fixture.installerWarningCodes,
    "installerWarningCodes",
    installerWarningCodes,
  );
  assertExactEnumArray(fixture.jobStages, "jobStages", jobStages);
  assertExactEnumArray(
    fixture.progressPhases,
    "progressPhases",
    progressPhases,
  );
  assertExactEnumArray(
    fixture.installerErrorCodes,
    "installerErrorCodes",
    installerErrorCodes,
  );
  assertExactEnumArray(
    fixture.suggestedActions,
    "suggestedActions",
    suggestedActions,
  );
  asArray(fixture.platformVersions, "platformVersions").forEach(
    (version, index) =>
      assertPlatformVersion(version, "platformVersions[" + index + "]"),
  );
  asArray(fixture.localInstallStatuses, "localInstallStatuses").forEach(
    (status, index) =>
      assertLocalInstallStatus(status, "localInstallStatuses[" + index + "]"),
  );
  const request = asRecord(fixture.startInstallRequest, "startInstallRequest");
  asString(request.expectedReleaseId, "startInstallRequest.expectedReleaseId");
  assertInstallResult(fixture.installResult, "installResult");
  assertJobSnapshot(fixture.jobSnapshot, "jobSnapshot");
}

describe("Codex desktop Rust/TypeScript DTO contract fixture", () => {
  it("parses every frozen enum branch and the full snapshot emitted by Rust", () => {
    const fixture: unknown = fixtureJson;
    assertContractFixture(fixture);

    expect(fixture.platformVersions.map((version) => version.kind)).toEqual([
      "windows_msix",
      "mac_bundle",
    ]);
    expect(fixture.localInstallStatuses.map((status) => status.state)).toEqual([
      "not_installed",
      "installed",
      "unsupported",
      "unsupported",
      "unsupported",
      "ambiguous",
    ]);
    expect(
      fixture.localInstallStatuses
        .filter(
          (
            status,
          ): status is Extract<LocalInstallStatus, { state: "unsupported" }> =>
            status.state === "unsupported",
        )
        .map((status) => status.reason),
    ).toEqual(unsupportedReasons);
    expect(fixture.startInstallRequest.expectedReleaseId).toMatch(
      /^v1:[a-f0-9]{64}$/,
    );
    expect(fixture.installResult.warnings).toEqual(installerWarningCodes);
    expect(fixture.jobSnapshot.stage).toBe("failed");
    expect(fixture.jobSnapshot.error?.code).toBe("DOWNLOAD_FAILED");
    expect(fixture.jobSnapshot.progress?.percent).toBe(50);
    expect(fixture.jobSnapshot.result).toBeNull();
  });
});
