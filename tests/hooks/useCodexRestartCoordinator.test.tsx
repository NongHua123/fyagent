import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCodexRestartCoordinator } from "@/hooks/useCodexRestartCoordinator";

const mocks = vi.hoisted(() => ({
  api: {
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
  mocks.api.requestRestart.mockReset().mockResolvedValue({
    state: "not_running",
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
  it("opens one confirmation from the backend-owned save follow-up", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-confirmation-token",
      reason: "unique_runtime",
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
      await result.current.notifyLiveConfigChanged();
    });

    expect(mocks.api.requestRestart).toHaveBeenCalledTimes(1);
    expect(result.current.dialog).toEqual({
      kind: "confirm",
      token: "opaque-confirmation-token",
      reason: "unique_runtime",
    });
    expect(mocks.toast.info).not.toHaveBeenCalled();
  });

  it("does not launch or show restart UI when Codex is not running", async () => {
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });

    expect(result.current.dialog).toBeNull();
    expect(mocks.api.continueRestartWithForce).not.toHaveBeenCalled();
    expect(mocks.toast.info).not.toHaveBeenCalled();
  });

  it("uses a non-destructive manual notice when automation is unavailable", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "manual_restart_required",
      reason: "untrusted_target",
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });

    expect(result.current.dialog).toBeNull();
    expect(mocks.toast.info).toHaveBeenCalledWith(
      "配置已保存；请手动重启 Codex 以加载新配置。",
    );
    expect(mocks.api.continueRestartWithForce).not.toHaveBeenCalled();
  });

  it("uses the confirmation token directly for force-close and restart without a second confirmation", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-confirmation-token",
      reason: "multiple_instances",
    });
    let resolveContinuation:
      | ((outcome: { state: "restarted" }) => void)
      | undefined;
    mocks.api.continueRestartWithForce.mockImplementation(
      () =>
        new Promise<{ state: "restarted" }>((resolve) => {
          resolveContinuation = resolve;
        }),
    );
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });

    let restart!: Promise<void>;
    await act(async () => {
      restart = result.current.requestRestart();
      await Promise.resolve();
    });

    expect(mocks.api.continueRestartWithForce).toHaveBeenCalledTimes(1);
    expect(mocks.api.continueRestartWithForce).toHaveBeenCalledWith(
      "opaque-confirmation-token",
    );
    expect(mocks.api.requestRestart).toHaveBeenCalledTimes(1);
    expect(result.current.dialog).toEqual({ kind: "progress" });
    expect(result.current.isRestarting).toBe(true);

    await act(async () => {
      resolveContinuation?.({ state: "restarted" });
      await restart;
    });

    expect(result.current.dialog).toBeNull();
    expect(mocks.toast.success).toHaveBeenCalledWith(
      "Codex 已重启，新配置已生效。",
    );
  });

  it("keeps a failed continuation in the incomplete dialog without diagnostic details", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-confirmation-token",
      reason: "identity_binding_ambiguous",
    });
    mocks.api.continueRestartWithForce.mockResolvedValue({
      state: "incomplete",
      retryToken: "opaque-retry-token",
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    expect(result.current.dialog).toEqual({
      kind: "incomplete",
      retryToken: "opaque-retry-token",
    });
    expect(mocks.toast.error).not.toHaveBeenCalled();
    expect(mocks.toast.info).not.toHaveBeenCalled();
  });

  it("does not reduce a post-confirmation fail-closed result to a toast", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-confirmation-token",
      reason: "unique_runtime",
    });
    mocks.api.continueRestartWithForce.mockResolvedValue({
      state: "manual_restart_required",
      reason: "untrusted_target",
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    expect(result.current.dialog).toEqual({ kind: "incomplete" });
    expect(mocks.toast.info).not.toHaveBeenCalled();
  });

  it("retries an incomplete operation with its opaque retry capability and no new confirmation", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-confirmation-token",
      reason: "multiple_installations",
    });
    mocks.api.continueRestartWithForce
      .mockResolvedValueOnce({
        state: "incomplete",
        retryToken: "opaque-retry-token",
      })
      .mockResolvedValueOnce({ state: "restarted" });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });
    await act(async () => {
      await result.current.retryRestart();
    });

    expect(mocks.api.continueRestartWithForce).toHaveBeenNthCalledWith(
      1,
      "opaque-confirmation-token",
    );
    expect(mocks.api.continueRestartWithForce).toHaveBeenNthCalledWith(
      2,
      "opaque-retry-token",
    );
    expect(mocks.api.requestRestart).toHaveBeenCalledTimes(1);
    expect(result.current.dialog).toBeNull();
  });

  it("quietly closes and best-effort discards the active capability", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-token-to-discard",
      reason: "unique_runtime",
    });
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      result.current.deferRestart();
    });

    expect(result.current.dialog).toBeNull();
    expect(mocks.api.cancelRestartWithForce).toHaveBeenCalledWith(
      "opaque-token-to-discard",
    );
    expect(mocks.toast.info).not.toHaveBeenCalled();
    expect(mocks.toast.error).not.toHaveBeenCalled();
  });

  it("uses the incomplete dialog rather than a toast when IPC execution rejects", async () => {
    mocks.api.requestRestart.mockResolvedValue({
      state: "confirmation_required",
      token: "opaque-confirmation-token",
      reason: "unique_runtime",
    });
    mocks.api.continueRestartWithForce.mockRejectedValue(
      new Error("fixture transport failure"),
    );
    const { result } = renderHook(() => useCodexRestartCoordinator());

    await act(async () => {
      await result.current.notifyLiveConfigChanged();
    });
    await act(async () => {
      await result.current.requestRestart();
    });

    expect(result.current.dialog).toEqual({ kind: "incomplete" });
    expect(mocks.toast.error).not.toHaveBeenCalled();
  });
});
