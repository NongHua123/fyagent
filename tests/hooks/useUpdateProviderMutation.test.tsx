import type { ReactNode } from "react";
import { act, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useUpdateProviderMutation } from "@/lib/query/mutations";
import { usageKeys } from "@/lib/query/usage";
import type { Provider } from "@/types";

const apiMocks = vi.hoisted(() => ({
  update: vi.fn(),
  updateWithResult: vi.fn(),
}));

const toastMocks = vi.hoisted(() => ({
  success: vi.fn(),
  warning: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    update: (...args: unknown[]) => apiMocks.update(...args),
    updateWithResult: (...args: unknown[]) =>
      apiMocks.updateWithResult(...args),
  },
  sessionsApi: {},
  settingsApi: {},
}));

vi.mock("@/hooks/useHermes", () => ({
  invalidateHermesProviderCaches: vi.fn(),
}));

vi.mock("@/hooks/useOpenClaw", () => ({
  openclawKeys: {
    health: ["openclaw", "health"],
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, options?: { defaultValue?: string }) =>
      options?.defaultValue ?? _key,
  }),
}));

vi.mock("sonner", () => ({
  toast: toastMocks,
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  return { wrapper, invalidateSpy };
}

function createProvider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: "provider-1",
    name: "Test Provider",
    settingsConfig: {},
    ...overrides,
  };
}

beforeEach(() => {
  apiMocks.update.mockReset().mockResolvedValue(true);
  apiMocks.updateWithResult
    .mockReset()
    .mockResolvedValue({ value: true, liveConfigChanged: false, app: "codex" });
  toastMocks.success.mockReset();
  toastMocks.warning.mockReset();
  toastMocks.error.mockReset();
});

describe("useUpdateProviderMutation", () => {
  it("invalidates the updated provider usage query", async () => {
    const { wrapper, invalidateSpy } = createWrapper();
    const provider = createProvider({ id: "provider-b" });
    const { result } = renderHook(() => useUpdateProviderMutation("codex"), {
      wrapper,
    });

    await act(async () => {
      await result.current.mutateAsync({ provider });
    });

    expect(apiMocks.updateWithResult).toHaveBeenCalledWith(
      provider,
      "codex",
      undefined,
    );
    expect(apiMocks.update).not.toHaveBeenCalled();
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["providers", "codex"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: usageKeys.script("provider-b", "codex"),
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: usageKeys.all,
    });
  });

  it("preserves the backend's Codex live-config result", async () => {
    apiMocks.updateWithResult.mockResolvedValueOnce({
      value: true,
      liveConfigChanged: true,
      app: "codex",
    });
    const { wrapper } = createWrapper();
    const provider = createProvider();
    const { result } = renderHook(() => useUpdateProviderMutation("codex"), {
      wrapper,
    });

    const outcome = await act(async () =>
      result.current.mutateAsync({ provider }),
    );

    expect(outcome).toEqual({ value: provider, liveConfigChanged: true });
    expect(toastMocks.success).toHaveBeenCalledTimes(1);
    expect(toastMocks.warning).not.toHaveBeenCalled();
  });

  it("also invalidates the previous usage query when provider id changes", async () => {
    const { wrapper, invalidateSpy } = createWrapper();
    const provider = createProvider({ id: "provider-new" });
    const { result } = renderHook(() => useUpdateProviderMutation("openclaw"), {
      wrapper,
    });

    await act(async () => {
      await result.current.mutateAsync({
        provider,
        originalId: "provider-old",
      });
    });

    expect(apiMocks.update).toHaveBeenCalledWith(
      provider,
      "openclaw",
      "provider-old",
    );
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: usageKeys.script("provider-new", "openclaw"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: usageKeys.script("provider-old", "openclaw"),
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: usageKeys.all,
    });
  });

  it("shows the backend WebSocket risk after every successful risky update", async () => {
    apiMocks.updateWithResult.mockResolvedValue({
      value: true,
      liveConfigChanged: false,
      app: "codex",
      warningCodes: ["CODEX_WEBSOCKET_NON_GPT_MODEL"],
    });
    const { wrapper } = createWrapper();
    const provider = createProvider();
    const { result } = renderHook(() => useUpdateProviderMutation("codex"), {
      wrapper,
    });

    await act(async () => {
      await result.current.mutateAsync({ provider });
      await result.current.mutateAsync({ provider });
    });

    expect(toastMocks.warning).toHaveBeenCalledTimes(2);
    expect(toastMocks.warning).toHaveBeenNthCalledWith(
      1,
      "供应商已保存；WebSocket 传输仅支持 GPT 系列模型",
      { closeButton: true },
    );
    expect(toastMocks.success).not.toHaveBeenCalled();
  });

  it("shows only the save error when the backend mutation fails", async () => {
    apiMocks.updateWithResult.mockRejectedValueOnce(new Error("save failed"));
    const { wrapper } = createWrapper();
    const provider = createProvider();
    const { result } = renderHook(() => useUpdateProviderMutation("codex"), {
      wrapper,
    });

    await act(async () => {
      await expect(result.current.mutateAsync({ provider })).rejects.toThrow(
        "save failed",
      );
    });

    expect(toastMocks.error).toHaveBeenCalledTimes(1);
    expect(toastMocks.success).not.toHaveBeenCalled();
    expect(toastMocks.warning).not.toHaveBeenCalled();
  });
});
