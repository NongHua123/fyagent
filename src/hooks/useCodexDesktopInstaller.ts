import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  blocksInstallOrUpdate,
  canRetryRemoteVersion,
  deriveLocalVersionState,
  deriveRemoteVersionState,
  type LocalVersionState,
  type RemoteVersionState,
} from "@/components/codex/versionState";
import { codexDesktopApi } from "@/lib/api/codex-desktop";
import {
  codexDesktopKeys,
  useCodexDesktopJob,
  useCodexDesktopLatestRelease,
  useCodexDesktopLocalStatus,
} from "@/lib/query/codex-desktop";
import {
  comparePlatformVersions,
  isInstalledLocalStatus,
  isTerminalJobStage,
  type InstallerErrorDto,
  type InstallerPrimaryAction,
  type InstallerViewState,
  type JobSnapshot,
  type LocalInstallStatus,
  type RemoteReleaseStatus,
} from "@/types/codexDesktop";

const JOB_UPDATED_EVENT = "codex-desktop-installer://job-updated";
const successToastJobIds = new Set<string>();

export interface CodexDesktopProgress {
  current: number | null;
  total: number | null;
  percent: number | null;
  bytesPerSecond: number | null;
}

interface DownloadSpeedSample {
  jobId: string;
  completedBytes: number;
  updatedAtMs: number;
}

interface DownloadSpeedMeasurement {
  jobId: string;
  sequence: number;
  bytesPerSecond: number;
}

function deriveDownloadSpeed(
  previous: DownloadSpeedSample | null,
  job: JobSnapshot | null | undefined,
): {
  sample: DownloadSpeedSample | null;
  bytesPerSecond: number | null;
} {
  const completedBytes = job?.progress?.completedBytes;
  const updatedAtMs = job ? Date.parse(job.updatedAt) : Number.NaN;
  if (
    job?.stage !== "downloading" ||
    job.progress?.phase !== "download" ||
    completedBytes == null ||
    !Number.isFinite(completedBytes) ||
    completedBytes < 0 ||
    !Number.isFinite(updatedAtMs)
  ) {
    return { sample: null, bytesPerSecond: null };
  }

  const sample = { jobId: job.jobId, completedBytes, updatedAtMs };
  if (!previous || previous.jobId !== sample.jobId) {
    return { sample, bytesPerSecond: null };
  }

  const elapsedMs = sample.updatedAtMs - previous.updatedAtMs;
  const completedDelta = sample.completedBytes - previous.completedBytes;
  if (elapsedMs <= 0 || completedDelta <= 0) {
    return { sample, bytesPerSecond: null };
  }

  const bytesPerSecond = (completedDelta * 1000) / elapsedMs;
  return {
    sample,
    bytesPerSecond:
      Number.isFinite(bytesPerSecond) && bytesPerSecond >= 0
        ? bytesPerSecond
        : null,
  };
}

export interface CodexDesktopInstallerViewModel {
  state: InstallerViewState;
  localVersion: LocalVersionState;
  remoteVersion: RemoteVersionState;
  canInstall: boolean;
  canUpdate: boolean;
  canLaunch: boolean;
  canRetryRemote: boolean;
  statusMessageKey: string;
  progress?: CodexDesktopProgress;
  primaryAction: InstallerPrimaryAction;
  primaryDisabled: boolean;
  canCancel: boolean;
  error: InstallerErrorDto | null;
  isActing: boolean;
  isRefreshing: boolean;
  refresh(): Promise<void>;
  runPrimaryAction(): Promise<void>;
  launch(): Promise<void>;
  cancel(): Promise<void>;
  copyErrorDetails(): Promise<void>;
  openLogs(): Promise<void>;
}

/**
 * Accepts only a later snapshot for the same job. Distinct job IDs are ordered
 * by backend-issued start time so a delayed event for an older terminal job
 * cannot overwrite a newly started installation.
 */
export function shouldAcceptJobSnapshot(
  current: JobSnapshot | null | undefined,
  incoming: JobSnapshot,
): boolean {
  if (!current) return true;

  if (current.jobId === incoming.jobId) {
    return incoming.sequence > current.sequence;
  }

  const currentStartedAt = Date.parse(current.startedAt);
  const incomingStartedAt = Date.parse(incoming.startedAt);
  if (Number.isFinite(currentStartedAt) && Number.isFinite(incomingStartedAt)) {
    if (incomingStartedAt !== currentStartedAt) {
      return incomingStartedAt > currentStartedAt;
    }
  }

  return (
    isTerminalJobStage(current.stage) && !isTerminalJobStage(incoming.stage)
  );
}

export function deriveInstallerViewState(
  local: LocalInstallStatus | undefined,
  remote: RemoteReleaseStatus | undefined,
  options: {
    localPending: boolean;
    remotePending: boolean;
    localFailed: boolean;
    remoteFailed: boolean;
    job: JobSnapshot | null | undefined;
  },
): InstallerViewState {
  if (local?.state === "unsupported") {
    return local.reason === "platform" ? "hidden" : "unsupported_architecture";
  }

  if (local?.state === "ambiguous") {
    return "ambiguous";
  }

  const job = options.job;
  if (job) {
    switch (job.stage) {
      case "checking":
        return "job_checking";
      case "preflight":
        return "job_preflight";
      case "downloading":
        return "job_downloading";
      case "verifying_download":
        return "job_verifying_download";
      case "installing":
        return "job_installing";
      case "verifying_installation":
        return "job_verifying_installation";
      case "succeeded":
        return "succeeded";
      case "failed":
        return "failed";
      case "cancelled":
        return "cancelled";
    }
  }

  if (options.localPending || options.remotePending || !local) {
    return "checking";
  }

  // A background remote failure retains the previously validated descriptor.
  // Its dedicated version state disables install/update while the known local
  // application remains launchable; it must not collapse into unavailable.
  if (options.localFailed || !remote) {
    return isInstalledLocalStatus(local)
      ? "remote_unavailable_installed"
      : "remote_unavailable";
  }

  if (!isInstalledLocalStatus(local)) {
    return "ready_install";
  }

  const comparison = comparePlatformVersions(
    local.application.platformVersion,
    remote.platformVersion,
  );
  if (comparison === -1) return "ready_update";
  if (comparison === 0) return "ready_launch";
  if (comparison === 1) return "local_newer";

  return "remote_unavailable_installed";
}

function primaryActionFor(
  state: InstallerViewState,
  local: LocalInstallStatus | undefined,
  remote: RemoteReleaseStatus | undefined,
  error: InstallerErrorDto | null,
): InstallerPrimaryAction {
  switch (state) {
    case "ready_install":
      return "install";
    case "ready_update":
      return "update";
    case "ready_launch":
    case "local_newer":
    case "remote_unavailable_installed":
    case "succeeded":
      return "launch";
    case "checking":
      return isInstalledLocalStatus(local) ? "launch" : null;
    case "remote_unavailable":
      return "retry";
    case "failed":
      if (error?.suggestedAction === "refresh") return "refresh";
      return error?.retryable ? "retry" : null;
    case "cancelled":
      if (!remote) return "retry";
      if (!isInstalledLocalStatus(local)) return "install";
      return (
        primaryActionFor(
          deriveInstallerViewState(local, remote, {
            localPending: false,
            remotePending: false,
            localFailed: false,
            remoteFailed: false,
            job: null,
          }),
          local,
          remote,
          null,
        ) ?? "retry"
      );
    default:
      return null;
  }
}

const installerStateMessageKeys: Record<InstallerViewState, string> = {
  hidden: "codexDesktop.state.hidden",
  checking: "codexDesktop.state.checking",
  unsupported_architecture: "codexDesktop.state.unsupportedArchitecture",
  ambiguous: "codexDesktop.state.ambiguous",
  ready_install: "codexDesktop.state.readyInstall",
  ready_update: "codexDesktop.state.updateAvailable",
  ready_launch: "codexDesktop.state.upToDate",
  local_newer: "codexDesktop.state.localNewer",
  remote_unavailable: "codexDesktop.state.remoteUnavailable",
  remote_unavailable_installed: "codexDesktop.state.remoteUnavailableInstalled",
  job_checking: "codexDesktop.state.checking",
  job_preflight: "codexDesktop.state.preflight",
  job_downloading: "codexDesktop.state.downloading",
  job_verifying_download: "codexDesktop.state.verifyingDownload",
  job_installing: "codexDesktop.state.installing",
  job_verifying_installation: "codexDesktop.state.verifyingInstallation",
  succeeded: "codexDesktop.state.succeeded",
  failed: "codexDesktop.state.failed",
  cancelled: "codexDesktop.state.cancelled",
};

function statusMessageKeyFor(
  state: InstallerViewState,
  localVersion: LocalVersionState,
  remoteVersion: RemoteVersionState,
): string {
  if (
    state.startsWith("job_") ||
    state === "succeeded" ||
    state === "failed" ||
    state === "cancelled" ||
    state === "hidden" ||
    state === "unsupported_architecture" ||
    state === "ambiguous"
  ) {
    return installerStateMessageKeys[state];
  }

  switch (remoteVersion.kind) {
    case "refreshing":
      return "codexDesktop.version.refreshing";
    case "refetch_error":
      return "codexDesktop.version.refreshNetworkFailed";
    case "initial_network_error":
      return "codexDesktop.version.fetchFailed";
    case "platform_unavailable":
      return "codexDesktop.version.platformUnavailable";
    case "metadata_error":
      return "codexDesktop.version.metadataInvalid";
    case "loading":
      return localVersion.kind === "loading"
        ? "codexDesktop.version.localLoading"
        : "codexDesktop.version.remoteLoading";
    case "available":
      if (localVersion.kind === "loading") {
        return "codexDesktop.version.localLoading";
      }
      if (localVersion.kind === "error") {
        return "codexDesktop.version.localError";
      }
      return installerStateMessageKeys[state];
  }
}

function asInstallerError(error: unknown): InstallerErrorDto | null {
  if (!error || typeof error !== "object") return null;
  const candidate = error as Partial<InstallerErrorDto>;
  return typeof candidate.code === "string" &&
    typeof candidate.messageKey === "string" &&
    candidate.details
    ? (candidate as InstallerErrorDto)
    : null;
}

function latestKnownError(
  local: LocalInstallStatus | undefined,
  job: JobSnapshot | null | undefined,
  errors: unknown[],
): InstallerErrorDto | null {
  if (job?.error) return job.error;
  if (local?.state === "ambiguous") return local.error;

  for (const error of errors) {
    const installerError = asInstallerError(error);
    if (installerError) return installerError;
  }

  return null;
}

function errorDetailsForCopy(error: InstallerErrorDto | null): string | null {
  if (!error) return null;
  return JSON.stringify(error, null, 2);
}

export function useCodexDesktopInstaller(): CodexDesktopInstallerViewModel {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const localQuery = useCodexDesktopLocalStatus();
  const remoteQuery = useCodexDesktopLatestRelease();
  const jobQuery = useCodexDesktopJob();
  const [actionError, setActionError] = useState<unknown>(null);
  const [isActing, setIsActing] = useState(false);
  const downloadSpeedSampleRef = useRef<DownloadSpeedSample | null>(null);
  const [downloadSpeedMeasurement, setDownloadSpeedMeasurement] =
    useState<DownloadSpeedMeasurement | null>(null);
  const [acknowledgedMetadataChangeJobId, setAcknowledgedMetadataChangeJobId] =
    useState<string | null>(null);

  const mergeJobSnapshot = useCallback(
    (incoming: JobSnapshot) => {
      queryClient.setQueryData<JobSnapshot | null>(
        codexDesktopKeys.job(),
        (current) =>
          shouldAcceptJobSnapshot(current, incoming) ? incoming : current,
      );
    },
    [queryClient],
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    void (async () => {
      try {
        const dispose = await listen<JobSnapshot>(
          JOB_UPDATED_EVENT,
          (event) => {
            mergeJobSnapshot(event.payload);
          },
        );
        if (disposed) {
          dispose();
          return;
        }
        unlisten = dispose;
        const snapshot = await codexDesktopApi.getJob();
        if (!disposed && snapshot) {
          mergeJobSnapshot(snapshot);
        }
      } catch (error) {
        if (!disposed) {
          console.warn("Failed to recover Codex desktop installer job", error);
        }
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [mergeJobSnapshot]);

  useEffect(() => {
    const recoverOnFocus = () => {
      void codexDesktopApi
        .getJob()
        .then((snapshot) => {
          if (snapshot) mergeJobSnapshot(snapshot);
        })
        .catch((error) => {
          console.warn("Failed to refresh Codex desktop installer job", error);
        });
    };
    window.addEventListener("focus", recoverOnFocus);
    return () => window.removeEventListener("focus", recoverOnFocus);
  }, [mergeJobSnapshot]);

  const local = localQuery.data;
  const remote = remoteQuery.data;
  const job = jobQuery.data;
  const localVersion = deriveLocalVersionState(local, {
    isLoading: localQuery.isLoading,
    isError: localQuery.isError,
  });
  const remoteVersion = deriveRemoteVersionState(remote, {
    isLoading: remoteQuery.isLoading,
    isError: remoteQuery.isError,
    isRefetching: remoteQuery.isRefetching,
    isRefetchError: remoteQuery.isRefetchError,
    errorCode: asInstallerError(remoteQuery.error)?.code ?? null,
  });

  useLayoutEffect(() => {
    const next = deriveDownloadSpeed(downloadSpeedSampleRef.current, job);
    downloadSpeedSampleRef.current = next.sample;
    setDownloadSpeedMeasurement(
      job && next.bytesPerSecond != null
        ? {
            jobId: job.jobId,
            sequence: job.sequence,
            bytesPerSecond: next.bytesPerSecond,
          }
        : null,
    );
  }, [job]);

  const isAcknowledgedMetadataChange =
    job?.stage === "failed" && job.jobId === acknowledgedMetadataChangeJobId;
  // JobStore intentionally retains terminal successes. Once a refresh reports
  // another release, local and remote versions determine the next action.
  const isSucceededJobSupersededByRemote =
    job?.stage === "succeeded" &&
    remote !== undefined &&
    job.release.releaseId !== remote.releaseId;
  const displayJob =
    isAcknowledgedMetadataChange || isSucceededJobSupersededByRemote
      ? null
      : job;
  const state = deriveInstallerViewState(local, remote, {
    localPending: localQuery.isLoading,
    remotePending: remoteQuery.isLoading,
    localFailed: localQuery.isError,
    remoteFailed: remoteQuery.isError,
    job: displayJob,
  });
  const error = latestKnownError(local, displayJob, [
    actionError,
    localQuery.error,
    remoteQuery.error,
  ]);
  const defaultPrimaryAction = primaryActionFor(state, local, remote, error);
  const canInstall =
    localVersion.kind === "not_installed" && remoteVersion.kind === "available";
  const canUpdate =
    isInstalledLocalStatus(local) &&
    remoteVersion.kind === "available" &&
    remote !== undefined &&
    comparePlatformVersions(
      local.application.platformVersion,
      remote.platformVersion,
    ) === -1;
  const canLaunch = localVersion.kind === "installed";
  const canRetryRemote = canRetryRemoteVersion(remoteVersion);
  const primaryAction =
    state === "remote_unavailable" && !canRetryRemote
      ? null
      : defaultPrimaryAction;
  const statusMessageKey = statusMessageKeyFor(
    state,
    localVersion,
    remoteVersion,
  );

  useEffect(() => {
    if (!job || !isTerminalJobStage(job.stage)) return;
    void queryClient.invalidateQueries({ queryKey: codexDesktopKeys.local() });
    void queryClient.invalidateQueries({ queryKey: codexDesktopKeys.remote() });
  }, [job?.jobId, job?.stage, queryClient]);

  useEffect(() => {
    if (job?.stage !== "succeeded" || successToastJobIds.has(job.jobId)) {
      return;
    }
    successToastJobIds.add(job.jobId);
    toast.success(t("codexDesktop.toast.installed"));
  }, [job?.jobId, job?.stage, t]);

  const refreshLatest = useCallback(async (): Promise<boolean> => {
    setActionError(null);
    try {
      const latest = await queryClient.fetchQuery({
        queryKey: codexDesktopKeys.remote(),
        queryFn: () => codexDesktopApi.checkLatest(true),
        staleTime: 0,
      });
      queryClient.setQueryData(codexDesktopKeys.remote(), latest);
      return true;
    } catch (error) {
      setActionError(error);
      return false;
    }
  }, [queryClient]);

  const refresh = useCallback(async () => {
    const refreshed = await refreshLatest();
    if (
      refreshed &&
      job?.stage === "failed" &&
      job.error?.suggestedAction === "refresh"
    ) {
      // A metadata mismatch is deliberately a two-step action: refreshing
      // reveals the newly checked release, while a separate primary action is
      // required before any installation can start.
      setAcknowledgedMetadataChangeJobId(job.jobId);
    }
  }, [job, refreshLatest]);

  const startWithKnownRelease = useCallback(async () => {
    const expectedReleaseId = remote?.releaseId ?? job?.release.releaseId;
    if (!expectedReleaseId) {
      await refresh();
      return;
    }

    const snapshot = await codexDesktopApi.startInstall(expectedReleaseId);
    mergeJobSnapshot(snapshot);
  }, [job?.release.releaseId, mergeJobSnapshot, refresh, remote?.releaseId]);

  const launch = useCallback(async () => {
    if (isActing) return;
    setActionError(null);
    setIsActing(true);
    try {
      await codexDesktopApi.launch();
    } catch (error) {
      setActionError(error);
    } finally {
      setIsActing(false);
    }
  }, [isActing]);

  const runPrimaryAction = useCallback(async () => {
    if (!primaryAction || isActing) return;
    setActionError(null);
    setIsActing(true);
    try {
      if (primaryAction === "launch") {
        await codexDesktopApi.launch();
      } else if (
        primaryAction === "refresh" ||
        (primaryAction === "retry" && state === "remote_unavailable")
      ) {
        await refresh();
      } else {
        await startWithKnownRelease();
      }
    } catch (error) {
      setActionError(error);
    } finally {
      setIsActing(false);
    }
  }, [isActing, primaryAction, refresh, startWithKnownRelease, state]);

  const cancel = useCallback(async () => {
    if (!job?.cancellable || isActing) return;
    setActionError(null);
    setIsActing(true);
    try {
      const snapshot = await codexDesktopApi.cancelInstall(job.jobId);
      mergeJobSnapshot(snapshot);
    } catch (error) {
      setActionError(error);
    } finally {
      setIsActing(false);
    }
  }, [isActing, job?.cancellable, job?.jobId, mergeJobSnapshot]);

  const copyErrorDetails = useCallback(async () => {
    const details = errorDetailsForCopy(error);
    if (!details) return;

    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("clipboard API is unavailable");
      }
      await navigator.clipboard.writeText(details);
      toast.success(t("codexDesktop.toast.copied"));
    } catch (clipboardError) {
      console.warn(
        "Failed to copy Codex desktop installer diagnostics",
        clipboardError,
      );
      toast.error(t("codexDesktop.toast.copyFailed"));
    }
  }, [error, t]);

  const openLogs = useCallback(async () => {
    setActionError(null);
    setIsActing(true);
    try {
      await codexDesktopApi.openLogDirectory();
    } catch (openLogsError) {
      setActionError(openLogsError);
    } finally {
      setIsActing(false);
    }
  }, []);

  const downloadBytesPerSecond =
    job?.stage === "downloading" &&
    job.progress?.phase === "download" &&
    downloadSpeedMeasurement?.jobId === job.jobId &&
    downloadSpeedMeasurement.sequence === job.sequence
      ? downloadSpeedMeasurement.bytesPerSecond
      : null;

  const progress = useMemo<CodexDesktopProgress | undefined>(() => {
    if (!job?.progress) return undefined;
    return {
      current: job.progress.completedBytes,
      total: job.progress.totalBytes,
      percent: job.progress.percent,
      bytesPerSecond: downloadBytesPerSecond,
    };
  }, [downloadBytesPerSecond, job?.progress]);

  return {
    state,
    localVersion,
    remoteVersion,
    canInstall,
    canUpdate,
    canLaunch,
    canRetryRemote,
    statusMessageKey,
    progress,
    primaryAction,
    primaryDisabled:
      !primaryAction ||
      isActing ||
      ((primaryAction === "install" || primaryAction === "update") &&
        (blocksInstallOrUpdate(remoteVersion) ||
          (primaryAction === "install" && !canInstall) ||
          (primaryAction === "update" && !canUpdate))),
    canCancel: Boolean(job?.cancellable) && !isActing,
    error,
    isActing,
    isRefreshing: remoteQuery.isFetching,
    refresh,
    runPrimaryAction,
    launch,
    cancel,
    copyErrorDetails,
    openLogs,
  };
}
