import type { ReactNode } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  InstallerErrorDto,
  JobSnapshot,
  LocalInstallStatus,
  RemoteReleaseStatus,
} from "@/types/codexDesktop";
import {
  deriveInstallerViewState,
  shouldAcceptJobSnapshot,
  useCodexDesktopInstaller,
} from "@/hooks/useCodexDesktopInstaller";
import { codexDesktopKeys } from "@/lib/query/codex-desktop";
import { createTestQueryClient } from "../utils/testQueryClient";

const mocks = vi.hoisted(() => {
  const listeners = new Set<(event: { payload: JobSnapshot }) => void>();
  return {
    listeners,
    api: {
      getLocalStatus: vi.fn(),
      checkLatest: vi.fn(),
      getJob: vi.fn(),
      startInstall: vi.fn(),
      cancelInstall: vi.fn(),
      launch: vi.fn(),
      openLogDirectory: vi.fn(),
    },
    listen: vi.fn(
      async (
        _event: string,
        handler: (event: { payload: JobSnapshot }) => void,
      ) => {
        listeners.add(handler);
        return () => listeners.delete(handler);
      },
    ),
    toastSuccess: vi.fn(),
    toastError: vi.fn(),
  };
});

vi.mock("@/lib/api/codex-desktop", () => ({
  codexDesktopApi: mocks.api,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (event: string, handler: (event: { payload: JobSnapshot }) => void) =>
    mocks.listen(event, handler),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => mocks.toastSuccess(...args),
    error: (...args: unknown[]) => mocks.toastError(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) =>
      typeof options?.defaultValue === "string" ? options.defaultValue : key,
  }),
}));

const remote: RemoteReleaseStatus = {
  releaseId: "v1:" + "a".repeat(64),
  displayVersion: "26.1.0",
  platformVersion: {
    kind: "windows_msix",
    major: 26,
    minor: 1,
    build: 0,
    revision: 0,
  },
  expectedSize: 1024,
  checkedAt: "2026-07-29T00:00:00.000Z",
};

const notInstalled: LocalInstallStatus = {
  state: "not_installed",
  platform: "windows",
  architecture: "x86_64",
};

const installedOld: LocalInstallStatus = {
  state: "installed",
  application: {
    stableIdentity: "OpenAI.Codex",
    displayName: "ChatGPT",
    displayVersion: "26.0.0",
    platformVersion: {
      kind: "windows_msix",
      major: 26,
      minor: 0,
      build: 0,
      revision: 0,
    },
    architecture: "x86_64",
  },
};

const installedSame: LocalInstallStatus = {
  ...installedOld,
  application: {
    ...installedOld.application,
    displayVersion: "26.1.0",
    platformVersion: remote.platformVersion,
  },
};

const installedNewer: LocalInstallStatus = {
  ...installedOld,
  application: {
    ...installedOld.application,
    displayVersion: "26.2.0",
    platformVersion: {
      kind: "windows_msix",
      major: 26,
      minor: 2,
      build: 0,
      revision: 0,
    },
  },
};

function makeJob(
  stage: JobSnapshot["stage"],
  sequence: number,
  overrides: Partial<JobSnapshot> = {},
): JobSnapshot {
  return {
    jobId: "job-1",
    sequence,
    stage,
    release: remote,
    startedAt: "2026-07-29T00:00:00.000Z",
    updatedAt: "2026-07-29T00:00:00.000Z",
    progress: null,
    cancellable: stage === "checking" || stage === "downloading",
    result: null,
    error: null,
    ...overrides,
  };
}

function makeDownloadJob(
  sequence: number,
  completedBytes: number,
  updatedAt: string,
  overrides: Partial<JobSnapshot> = {},
): JobSnapshot {
  return makeJob("downloading", sequence, {
    updatedAt,
    progress: {
      phase: "download",
      completedBytes,
      totalBytes: 8 * 1024 * 1024,
      percent: 50,
    },
    ...overrides,
  });
}

function makeMetadataChangedError(): InstallerErrorDto {
  return {
    code: "METADATA_CHANGED",
    stage: "checking",
    messageKey: "codexDesktop.error.metadataChanged",
    retryable: true,
    suggestedAction: "refresh",
    details: {
      endpointKind: "metadata",
      attempt: null,
      maxAttempts: null,
      httpStatus: null,
      platformErrorCode: null,
      redactedMessage: "release metadata changed",
      context: {},
    },
  };
}

function makeRedactedDownloadError(): InstallerErrorDto {
  return {
    code: "DOWNLOAD_FAILED",
    stage: "downloading",
    messageKey: "codexDesktop.error.downloadFailed",
    retryable: true,
    suggestedAction: "retry",
    details: {
      endpointKind: "artifact",
      attempt: 3,
      maxAttempts: 3,
      httpStatus: 503,
      platformErrorCode: "HTTP_503",
      redactedMessage: "GET https://mirror.example/releases?[REDACTED] failed",
      context: { source: "agentsmirror" },
    },
  };
}

function emitJob(snapshot: JobSnapshot) {
  for (const listener of mocks.listeners) {
    listener({ payload: snapshot });
  }
}

function createWrapper() {
  const queryClient = createTestQueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { queryClient, wrapper };
}

beforeEach(() => {
  mocks.listeners.clear();
  mocks.listen.mockClear();
  mocks.toastSuccess.mockReset();
  mocks.toastError.mockReset();
  Object.values(mocks.api).forEach((mock) => mock.mockReset());
  mocks.api.getLocalStatus.mockResolvedValue(notInstalled);
  mocks.api.checkLatest.mockResolvedValue(remote);
  mocks.api.getJob.mockResolvedValue(null);
  mocks.api.startInstall.mockResolvedValue(makeJob("checking", 0));
  mocks.api.cancelInstall.mockResolvedValue(makeJob("cancelled", 1));
  mocks.api.launch.mockResolvedValue(undefined);
  mocks.api.openLogDirectory.mockResolvedValue(undefined);
});

describe("deriveInstallerViewState", () => {
  it("uses the canonical DTO versions only to choose visible install/update/launch states", () => {
    expect(
      deriveInstallerViewState(notInstalled, remote, {
        localPending: false,
        remotePending: false,
        localFailed: false,
        remoteFailed: false,
        job: null,
      }),
    ).toBe("ready_install");
    expect(
      deriveInstallerViewState(installedOld, remote, {
        localPending: false,
        remotePending: false,
        localFailed: false,
        remoteFailed: false,
        job: null,
      }),
    ).toBe("ready_update");
    expect(
      deriveInstallerViewState(installedOld, undefined, {
        localPending: false,
        remotePending: false,
        localFailed: false,
        remoteFailed: true,
        job: null,
      }),
    ).toBe("remote_unavailable_installed");
  });

  it("maps equal and locally newer canonical versions to launch states", () => {
    const readyOptions = {
      localPending: false,
      remotePending: false,
      localFailed: false,
      remoteFailed: false,
      job: null,
    };

    expect(deriveInstallerViewState(installedSame, remote, readyOptions)).toBe(
      "ready_launch",
    );
    expect(deriveInstallerViewState(installedNewer, remote, readyOptions)).toBe(
      "local_newer",
    );
  });

  it("keeps Linux hidden and Intel Mac visibly unsupported", () => {
    expect(
      deriveInstallerViewState(
        { state: "unsupported", reason: "platform" },
        undefined,
        {
          localPending: false,
          remotePending: false,
          localFailed: false,
          remoteFailed: false,
          job: null,
        },
      ),
    ).toBe("hidden");
    expect(
      deriveInstallerViewState(
        { state: "unsupported", reason: "architecture" },
        undefined,
        {
          localPending: false,
          remotePending: false,
          localFailed: false,
          remoteFailed: false,
          job: null,
        },
      ),
    ).toBe("unsupported_architecture");
  });
});

describe("shouldAcceptJobSnapshot", () => {
  it("uses sequence for the same job and start time for different jobs", () => {
    const current = makeJob("downloading", 3);
    expect(shouldAcceptJobSnapshot(current, makeJob("checking", 2))).toBe(
      false,
    );
    expect(shouldAcceptJobSnapshot(current, makeJob("downloading", 4))).toBe(
      true,
    );
    expect(
      shouldAcceptJobSnapshot(
        current,
        makeJob("checking", 0, {
          jobId: "new-job",
          startedAt: "2026-07-29T00:01:00.000Z",
        }),
      ),
    ).toBe(true);
  });
});

describe("useCodexDesktopInstaller", () => {
  it("registers its listener before recovering the current job and drops an older query snapshot", async () => {
    let resolveJob: ((snapshot: JobSnapshot | null) => void) | undefined;
    mocks.api.getJob.mockImplementation(
      () =>
        new Promise<JobSnapshot | null>((resolve) => {
          resolveJob = resolve;
        }),
    );
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(mocks.listeners.size).toBe(1));
    expect(mocks.listen.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.api.getJob.mock.invocationCallOrder[0],
    );

    await act(async () => {
      emitJob(makeJob("downloading", 2));
      resolveJob?.(makeJob("checking", 1));
    });

    await waitFor(() => expect(result.current.state).toBe("job_downloading"));
  });

  it("derives download speed only after two valid samples for the same job", async () => {
    const mebibyte = 1024 * 1024;
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(mocks.listeners.size).toBe(1));
    await act(async () => {
      emitJob(makeDownloadJob(1, mebibyte, "2026-07-29T00:00:01.000Z"));
    });

    await waitFor(() => {
      expect(result.current.progress?.current).toBe(mebibyte);
      expect(result.current.progress?.bytesPerSecond).toBeNull();
    });

    await act(async () => {
      emitJob(makeDownloadJob(2, 5 * mebibyte, "2026-07-29T00:00:03.000Z"));
    });

    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond).toBe(2 * mebibyte),
    );
  });

  it("resets a zero-byte interval without exposing stale speed and recovers from that sample", async () => {
    const mebibyte = 1024 * 1024;
    const observedProgress: Array<{
      current: number | null | undefined;
      bytesPerSecond: number | null | undefined;
    }> = [];
    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => {
        const installer = useCodexDesktopInstaller();
        observedProgress.push({
          current: installer.progress?.current,
          bytesPerSecond: installer.progress?.bytesPerSecond,
        });
        return installer;
      },
      { wrapper },
    );

    await waitFor(() => expect(mocks.listeners.size).toBe(1));
    await act(async () => {
      emitJob(makeDownloadJob(1, mebibyte, "2026-07-29T00:00:01.000Z"));
    });
    await act(async () => {
      emitJob(makeDownloadJob(2, 3 * mebibyte, "2026-07-29T00:00:02.000Z"));
    });
    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond).toBe(2 * mebibyte),
    );

    const resetRenderStart = observedProgress.length;
    await act(async () => {
      emitJob(makeDownloadJob(3, 3 * mebibyte, "2026-07-29T00:00:03.000Z"));
    });
    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond).toBeNull(),
    );
    expect(observedProgress.slice(resetRenderStart)).not.toContainEqual({
      current: 3 * mebibyte,
      bytesPerSecond: 2 * mebibyte,
    });

    await act(async () => {
      emitJob(makeDownloadJob(4, 5 * mebibyte, "2026-07-29T00:00:04.000Z"));
    });
    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond).toBe(2 * mebibyte),
    );
  });

  it.each([
    {
      caseName: "the timestamp is invalid",
      snapshot: () => makeDownloadJob(3, 4 * 1024 * 1024, "not-a-timestamp"),
    },
    {
      caseName: "the byte count is not finite",
      snapshot: () =>
        makeDownloadJob(3, Number.NaN, "2026-07-29T00:00:03.000Z"),
    },
    {
      caseName: "time does not advance",
      snapshot: () =>
        makeDownloadJob(3, 4 * 1024 * 1024, "2026-07-29T00:00:02.000Z"),
    },
    {
      caseName: "the byte count moves backwards",
      snapshot: () =>
        makeDownloadJob(3, 512 * 1024, "2026-07-29T00:00:03.000Z"),
    },
    {
      caseName: "the progress phase is not download",
      snapshot: () =>
        makeDownloadJob(3, 4 * 1024 * 1024, "2026-07-29T00:00:03.000Z", {
          progress: {
            phase: "verification",
            completedBytes: 4 * 1024 * 1024,
            totalBytes: 8 * 1024 * 1024,
            percent: 50,
          },
        }),
    },
    {
      caseName: "download progress disappears",
      snapshot: () =>
        makeJob("downloading", 3, {
          updatedAt: "2026-07-29T00:00:03.000Z",
          progress: null,
          cancellable: false,
        }),
    },
    {
      caseName: "a new job starts",
      snapshot: () =>
        makeDownloadJob(1, 4 * 1024 * 1024, "2026-07-29T00:01:01.000Z", {
          jobId: "job-2",
          startedAt: "2026-07-29T00:01:00.000Z",
        }),
    },
    {
      caseName: "the job leaves the download stage",
      snapshot: () =>
        makeJob("installing", 3, {
          updatedAt: "2026-07-29T00:00:03.000Z",
          progress: {
            phase: "installation",
            completedBytes: 1,
            totalBytes: 2,
            percent: 50,
          },
          cancellable: false,
        }),
    },
  ])("clears download speed when $caseName", async ({ snapshot }) => {
    const mebibyte = 1024 * 1024;
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(mocks.listeners.size).toBe(1));
    await act(async () => {
      emitJob(makeDownloadJob(1, mebibyte, "2026-07-29T00:00:01.000Z"));
    });
    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond).toBeNull(),
    );

    await act(async () => {
      emitJob(makeDownloadJob(2, 3 * mebibyte, "2026-07-29T00:00:02.000Z"));
    });
    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond).toBe(2 * mebibyte),
    );

    await act(async () => {
      emitJob(snapshot());
    });

    await waitFor(() =>
      expect(result.current.progress?.bytesPerSecond ?? null).toBeNull(),
    );
  });

  it("does not cancel the backend job when the card unmounts", async () => {
    const { wrapper } = createWrapper();
    const { unmount } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(mocks.listeners.size).toBe(1));
    unmount();

    expect(mocks.api.cancelInstall).not.toHaveBeenCalled();
  });

  it("releases a listener that resolves after unmount before it can update the cache", async () => {
    const dispose = vi.fn();
    let resolveListen: (() => void) | undefined;
    mocks.listen.mockImplementationOnce(
      (_event, handler) =>
        new Promise<() => boolean>((resolve) => {
          resolveListen = () => {
            mocks.listeners.add(handler);
            resolve(() => {
              dispose();
              return mocks.listeners.delete(handler);
            });
          };
        }),
    );

    const { queryClient, wrapper } = createWrapper();
    const initialJob = makeJob("checking", 1);
    queryClient.setQueryData(codexDesktopKeys.job(), initialJob);
    const { unmount } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(mocks.listen).toHaveBeenCalledOnce());
    unmount();

    await act(async () => {
      resolveListen?.();
    });

    await waitFor(() => expect(dispose).toHaveBeenCalledOnce());
    expect(mocks.listeners.size).toBe(0);
    expect(mocks.api.getJob).not.toHaveBeenCalled();

    await act(async () => {
      emitJob(makeJob("downloading", 2));
    });

    expect(queryClient.getQueryData(codexDesktopKeys.job())).toEqual(
      initialJob,
    );
  });

  it("shows a success toast once for a job even when newer success snapshots arrive", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(mocks.listeners.size).toBe(1));
    await act(async () => {
      emitJob(makeJob("succeeded", 1, { jobId: "success-job" }));
      emitJob(makeJob("succeeded", 2, { jobId: "success-job" }));
    });

    await waitFor(() => expect(result.current.state).toBe("succeeded"));
    expect(mocks.toastSuccess).toHaveBeenCalledTimes(1);
  });

  it("returns to the version-derived update state when a refreshed release supersedes a successful job", async () => {
    const refreshedRemote: RemoteReleaseStatus = {
      ...remote,
      releaseId: "v1:" + "b".repeat(64),
      displayVersion: "26.2.0",
      platformVersion: {
        kind: "windows_msix",
        major: 26,
        minor: 2,
        build: 0,
        revision: 0,
      },
    };
    mocks.api.getLocalStatus.mockResolvedValue(installedSame);
    mocks.api.getJob.mockResolvedValue(
      makeJob("succeeded", 1, { jobId: "success-refresh-job" }),
    );
    mocks.api.checkLatest.mockImplementation((force: boolean) =>
      Promise.resolve(force ? refreshedRemote : remote),
    );

    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(result.current.state).toBe("succeeded"));
    expect(mocks.toastSuccess).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(result.current.isRefreshing).toBe(false));

    await act(async () => {
      await result.current.refresh();
    });

    await waitFor(() => {
      expect(result.current.state).toBe("ready_update");
      expect(result.current.primaryAction).toBe("update");
    });
    expect(mocks.toastSuccess).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.runPrimaryAction();
    });

    expect(mocks.api.startInstall).toHaveBeenCalledWith(
      refreshedRemote.releaseId,
    );
    expect(mocks.api.launch).not.toHaveBeenCalled();
  });

  it("starts an update with the displayed release ID and leaves launch to its own action", async () => {
    mocks.api.getLocalStatus.mockResolvedValue(installedOld);
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(result.current.state).toBe("ready_update"));
    await act(async () => {
      await result.current.runPrimaryAction();
    });

    expect(mocks.api.startInstall).toHaveBeenCalledWith(remote.releaseId);
    expect(mocks.api.launch).not.toHaveBeenCalled();
  });

  it.each([
    ["ready_launch", installedSame],
    ["local_newer", installedNewer],
  ] as const)(
    "launches without starting an install from the %s state",
    async (expectedState, localStatus) => {
      mocks.api.getLocalStatus.mockResolvedValue(localStatus);
      const { wrapper } = createWrapper();
      const { result } = renderHook(() => useCodexDesktopInstaller(), {
        wrapper,
      });

      await waitFor(() => expect(result.current.state).toBe(expectedState));
      expect(result.current.primaryAction).toBe("launch");

      await act(async () => {
        await result.current.runPrimaryAction();
      });

      expect(mocks.api.launch).toHaveBeenCalledOnce();
      expect(mocks.api.startInstall).not.toHaveBeenCalled();
    },
  );

  it("preserves launch when the remote check fails for an installed app", async () => {
    mocks.api.getLocalStatus.mockResolvedValue(installedOld);
    mocks.api.checkLatest.mockRejectedValue({
      ...makeRedactedDownloadError(),
      code: "SOURCE_UNAVAILABLE",
      stage: null,
      messageKey: "codexDesktop.error.sourceUnavailable",
    } satisfies InstallerErrorDto);
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() =>
      expect(result.current.state).toBe("remote_unavailable_installed"),
    );
    expect(result.current.primaryAction).toBe("launch");

    await act(async () => {
      await result.current.runPrimaryAction();
    });

    expect(mocks.api.launch).toHaveBeenCalledOnce();
    expect(mocks.api.startInstall).not.toHaveBeenCalled();
  });

  it.each(["installing", "verifying_installation"] as const)(
    "does not expose cancellation while a job is %s",
    async (stage) => {
      mocks.api.getJob.mockResolvedValue(
        makeJob(stage, 4, { cancellable: false }),
      );
      const { wrapper } = createWrapper();
      const { result } = renderHook(() => useCodexDesktopInstaller(), {
        wrapper,
      });

      await waitFor(() => expect(result.current.state).toBe(`job_${stage}`));
      expect(result.current.canCancel).toBe(false);
      expect(result.current.primaryAction).toBeNull();

      await act(async () => {
        await result.current.cancel();
      });
      expect(mocks.api.cancelInstall).not.toHaveBeenCalled();
    },
  );

  it("copies the backend-redacted structured error without reconstructing raw URLs", async () => {
    const error = makeRedactedDownloadError();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    mocks.api.getJob.mockResolvedValue(
      makeJob("failed", 4, { error, cancellable: false }),
    );
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(result.current.state).toBe("failed"));
    await act(async () => {
      await result.current.copyErrorDetails();
    });

    expect(writeText).toHaveBeenCalledWith(JSON.stringify(error, null, 2));
    const copiedText = writeText.mock.calls[0][0] as string;
    expect(copiedText).toContain("[REDACTED]");
    expect(copiedText).not.toContain("token=unredacted-secret");
    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      "codexDesktop.toast.copied",
    );
  });

  it("reports clipboard failures without replacing the installer error", async () => {
    const error = makeRedactedDownloadError();
    const writeText = vi.fn().mockRejectedValue(new Error("permission denied"));
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    mocks.api.getJob.mockResolvedValue(
      makeJob("failed", 5, { error, cancellable: false }),
    );
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(result.current.error).toEqual(error));
    await act(async () => {
      await result.current.copyErrorDetails();
    });

    expect(mocks.toastError).toHaveBeenCalledWith(
      "codexDesktop.toast.copyFailed",
    );
    expect(result.current.error).toEqual(error);
    expect(warn).toHaveBeenCalledOnce();
    warn.mockRestore();
  });

  it("requires a metadata refresh to finish before a separate action can install the newly checked release", async () => {
    const refreshedRemote: RemoteReleaseStatus = {
      ...remote,
      releaseId: "v1:" + "b".repeat(64),
      displayVersion: "26.2.0",
      platformVersion: {
        kind: "windows_msix",
        major: 26,
        minor: 2,
        build: 0,
        revision: 0,
      },
    };
    mocks.api.getLocalStatus.mockResolvedValue(installedOld);
    mocks.api.getJob.mockResolvedValue(
      makeJob("failed", 1, { error: makeMetadataChangedError() }),
    );
    mocks.api.checkLatest
      .mockResolvedValueOnce(remote)
      .mockResolvedValue(refreshedRemote);

    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useCodexDesktopInstaller(), {
      wrapper,
    });

    await waitFor(() => expect(result.current.state).toBe("failed"));
    expect(result.current.primaryAction).toBe("refresh");

    await act(async () => {
      await result.current.runPrimaryAction();
    });

    expect(mocks.api.startInstall).not.toHaveBeenCalled();
    await waitFor(() => expect(result.current.state).toBe("ready_update"));
    expect(result.current.primaryAction).toBe("update");

    await act(async () => {
      await result.current.runPrimaryAction();
    });

    expect(mocks.api.startInstall).toHaveBeenCalledWith(
      refreshedRemote.releaseId,
    );
  });
});
