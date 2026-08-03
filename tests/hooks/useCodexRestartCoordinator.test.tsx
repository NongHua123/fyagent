import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCodexRestartCoordinator } from "@/hooks/useCodexRestartCoordinator";

const mocks = vi.hoisted(() => ({
  api: {
    getRuntimeStatus: vi.fn(),
    requestRestart: vi.fn(),
    continueRestartWithForce: vi.fn(),
    cancelRestartWithForce: vi.fn(),
  },
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock("@/lib/api", () => ({
  codexDesktopApi: mocks.api,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) =>
      options?.defaultValue ?? key,
  }),
}));

vi.mock("sonner", () => ({
  toast: mocks.toast,
}));

beforeEach(() => {
  mocks.api.getRuntimeStatus.mockReset().mockResolvedValue({
    state: "not_running",
  });
  mocks.api.requestRestart.mockReset().mockResolvedValue({
    state: "restarted",
  });
  mocks.api.continueRestartWithForce.mockReset().mockResolvedValue({
    state: "restarted",
  });
  mocks.api.cancelRestartWithForce.mockReset().mockResolvedValue(undefined);
  mocks.toast.success.mockReset();
  mocks.toast.error.mockReset();
  mocks.toast.info.mockReset();
});

describe("useCodexRestartCoordinator", () => {
  it("opens only one restart offer for a trusted running instance", async () => {
    mocks.api.getRuntimeStatus.mockResolvedValue({ state: "running" });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
      await result.current.notifyLiveConfigChanged();
    });

    expect(mocks.api.getRuntimeStatus).toHaveBeenCalledTimes(1);
    expect(result.current.dialog).toEqual({ kind: "restart" });
  });

  it("does not start a not-running Codex desktop", async () => {
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });

    expect(result.current.dialog).toBeNull();
    expect(mocks.api.requestRestart).not.toHaveBeenCalled();
    expect(mocks.api.continueRestartWithForce).not.toHaveBeenCalled();
  });

  it("echoes only the opaque backend force token after a second confirmation", async () => {
    mocks.api.getRuntimeStatus.mockResolvedValue({ state: "running" });
    mocks.api.requestRestart.mockResolvedValue({
      state: "force_confirmation_required",
      token: "opaque-single-use-token",
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    expect(result.current.dialog).toEqual({
      kind: "force",
      token: "opaque-single-use-token",
    });

    await act(async () => {
      await result.current.confirmForceRestart();
    });

    expect(mocks.api.continueRestartWithForce).toHaveBeenCalledTimes(1);
    expect(mocks.api.continueRestartWithForce).toHaveBeenCalledWith(
      "opaque-single-use-token",
    );
    expect(mocks.toast.success).toHaveBeenCalledWith(
      "Codex 已重启，新配置已生效。",
    );
  });

  it("discards a deferred force token before allowing a later restart retry", async () => {
    mocks.api.getRuntimeStatus.mockResolvedValue({ state: "running" });
    mocks.api.requestRestart
      .mockResolvedValueOnce({
        state: "force_confirmation_required",
        token: "opaque-token-to-discard",
      })
      .mockResolvedValueOnce({ state: "restarted" });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });
    expect(result.current.dialog).toEqual({
      kind: "force",
      token: "opaque-token-to-discard",
    });

    await act(async () => {
      await result.current.deferRestart();
    });

    expect(mocks.api.cancelRestartWithForce).toHaveBeenCalledTimes(1);
    expect(mocks.api.cancelRestartWithForce).toHaveBeenCalledWith(
      "opaque-token-to-discard",
    );
    expect(result.current.dialog).toBeNull();
    expect(mocks.toast.info).toHaveBeenCalledWith(
      "配置已保存；请稍后手动重启 Codex。",
    );

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    expect(mocks.api.requestRestart).toHaveBeenCalledTimes(2);
    expect(mocks.api.continueRestartWithForce).not.toHaveBeenCalled();
  });

  it("keeps the force dialog authoritative until token cancellation completes", async () => {
    mocks.api.getRuntimeStatus.mockResolvedValue({ state: "running" });
    mocks.api.requestRestart.mockResolvedValue({
      state: "force_confirmation_required",
      token: "opaque-token-with-pending-cancellation",
    });
    let resolveCancellation: (() => void) | undefined;
    mocks.api.cancelRestartWithForce.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveCancellation = resolve;
        }),
    );
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    let deferredRestart!: Promise<void>;
    await act(async () => {
      deferredRestart = result.current.deferRestart();
      await Promise.resolve();
    });

    expect(mocks.api.cancelRestartWithForce).toHaveBeenCalledWith(
      "opaque-token-with-pending-cancellation",
    );
    expect(result.current.dialog).toEqual({
      kind: "force",
      token: "opaque-token-with-pending-cancellation",
    });
    expect(result.current.isRestarting).toBe(true);

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    expect(mocks.api.getRuntimeStatus).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveCancellation?.();
      await deferredRestart;
    });
    expect(result.current.dialog).toBeNull();
    expect(result.current.isRestarting).toBe(false);
  });

  it("keeps the saved configuration outcome distinct when restart fails", async () => {
    mocks.api.getRuntimeStatus.mockResolvedValue({ state: "running" });
    mocks.api.requestRestart.mockResolvedValue({
      state: "failed",
      phase: "launch",
      error: {
        code: "LAUNCH_FAILED",
        stage: null,
        messageKey: "codexDesktop.error.launchFailed",
        retryable: false,
        suggestedAction: "none",
        details: {
          endpointKind: null,
          attempt: null,
          maxAttempts: null,
          httpStatus: null,
          platformErrorCode: null,
          redactedMessage: null,
          context: {},
        },
      },
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    expect(result.current.dialog).toBeNull();
    expect(mocks.toast.error).toHaveBeenCalledWith(
      "配置已保存，但 Codex 重启失败。请手动重启后再继续。",
      expect.objectContaining({ description: expect.any(String) }),
    );
  });
});
