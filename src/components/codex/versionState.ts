import {
  displayPlatformVersion,
  isInstalledLocalStatus,
  type InstallerErrorCode,
  type LocalInstallStatus,
  type RemoteReleaseStatus,
} from "@/types/codexDesktop";

export type LocalVersionState =
  | { kind: "loading" }
  | { kind: "installed"; version: string }
  | { kind: "not_installed" }
  | { kind: "error" };

export type RemoteVersionState =
  | { kind: "loading" }
  | { kind: "available"; version: string }
  | { kind: "refreshing"; version: string }
  | { kind: "refetch_error"; version: string }
  | { kind: "initial_network_error" }
  | { kind: "platform_unavailable" }
  | { kind: "metadata_error" };

interface LocalVersionQueryState {
  isLoading: boolean;
  isError: boolean;
}

interface RemoteVersionQueryState {
  isLoading: boolean;
  isError: boolean;
  isRefetching: boolean;
  isRefetchError: boolean;
  errorCode: InstallerErrorCode | null;
}

export function deriveLocalVersionState(
  local: LocalInstallStatus | undefined,
  query: LocalVersionQueryState,
): LocalVersionState {
  if (isInstalledLocalStatus(local)) {
    return {
      kind: "installed",
      version:
        local.application.displayVersion ??
        displayPlatformVersion(local.application.platformVersion),
    };
  }

  if (local?.state === "not_installed") {
    return { kind: "not_installed" };
  }

  if (query.isLoading || (!local && !query.isError)) {
    return { kind: "loading" };
  }

  return { kind: "error" };
}

export function deriveRemoteVersionState(
  remote: RemoteReleaseStatus | undefined,
  query: RemoteVersionQueryState,
): RemoteVersionState {
  if (remote) {
    if (query.isRefetching) {
      return { kind: "refreshing", version: remote.displayVersion };
    }

    if (query.isRefetchError) {
      return { kind: "refetch_error", version: remote.displayVersion };
    }

    return { kind: "available", version: remote.displayVersion };
  }

  if (query.isLoading || !query.isError) {
    return { kind: "loading" };
  }

  if (query.errorCode === "RELEASE_NOT_AVAILABLE") {
    return { kind: "platform_unavailable" };
  }

  if (
    query.errorCode === "RELEASE_METADATA_INVALID" ||
    query.errorCode === "METADATA_CHANGED"
  ) {
    return { kind: "metadata_error" };
  }

  return { kind: "initial_network_error" };
}

export function blocksInstallOrUpdate(
  remoteVersion: RemoteVersionState,
): boolean {
  return (
    remoteVersion.kind === "refreshing" ||
    remoteVersion.kind === "refetch_error"
  );
}

export function canRetryRemoteVersion(
  remoteVersion: RemoteVersionState,
): boolean {
  return (
    remoteVersion.kind === "initial_network_error" ||
    remoteVersion.kind === "refetch_error" ||
    remoteVersion.kind === "metadata_error"
  );
}
