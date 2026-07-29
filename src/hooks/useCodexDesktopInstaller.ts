import { useCallback, useEffect, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { codexDesktopApi } from "@/lib/api/codex-desktop";
import {
  codexDesktopKeys,
  useCodexDesktopJob,
  useCodexDesktopLatestRelease,
  useCodexDesktopLocalStatus,
} from "@/lib/query/codex-desktop";
import {
  comparePlatformVersions,
  displayPlatformVersion,
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
}

export interface CodexDesktopInstallerViewModel {
  state: InstallerViewState;
  localVersion?: string;
  remoteVersion?: string;
  progress?: CodexDesktopProgress;
  primaryAction: InstallerPrimaryAction;
  primaryDisabled: boolean;
  canCancel: boolean;
  error: InstallerErrorDto | null;
  isRefreshing: boolean;
  refresh(): Promise<void>;
  runPrimaryAction(): Promise<void>;
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

  if (options.localFailed || options.remoteFailed || !remote) {
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
  const primaryAction = primaryActionFor(state, local, remote, error);

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

  const progress = useMemo<CodexDesktopProgress | undefined>(() => {
    if (!job?.progress) return undefined;
    return {
      current: job.progress.completedBytes,
      total: job.progress.totalBytes,
      percent: job.progress.percent,
    };
  }, [job?.progress]);

  return {
    state,
    localVersion: isInstalledLocalStatus(local)
      ? (local.application.displayVersion ??
        displayPlatformVersion(local.application.platformVersion))
      : undefined,
    remoteVersion: remote?.displayVersion,
    progress,
    primaryAction,
    primaryDisabled: !primaryAction || isActing,
    canCancel: Boolean(job?.cancellable) && !isActing,
    error,
    isRefreshing: remoteQuery.isFetching,
    refresh,
    runPrimaryAction,
    cancel,
    copyErrorDetails,
    openLogs,
  };
}
