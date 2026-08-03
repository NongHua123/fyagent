import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useCodexProviderFeatures } from "@/hooks/useCodexProviderFeatures";
import type { CodexProviderFeatureState } from "@/lib/api";
import type { CodexApiFormat, Provider } from "@/types";

const apiMocks = vi.hoisted(() => ({
  analyzeCodexProviderFeatures: vi.fn(),
  patchCodexProviderFeatures: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  providersApi: apiMocks,
}));

const featureState = (
  overrides: Partial<CodexProviderFeatureState> = {},
): CodexProviderFeatureState => ({
  applicable: true,
  imageExtension: { kind: "off" },
  websockets: { enabled: false, compatible: true },
  providerTableFound: true,
  diagnostics: [],
  ...overrides,
});

const draftFor = (
  config: string,
  apiFormat: CodexApiFormat = "openai_responses",
): Provider => ({
  id: "provider-1",
  name: "Third-party provider",
  category: "custom",
  settingsConfig: {
    auth: { OPENAI_API_KEY: "test-only" },
    config,
  },
  meta: { apiFormat },
});

beforeEach(() => {
  vi.useFakeTimers();
  apiMocks.analyzeCodexProviderFeatures.mockReset();
  apiMocks.patchCodexProviderFeatures.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useCodexProviderFeatures", () => {
  it("removes WebSocket from the TOML draft when leaving Responses", async () => {
    const onTomlPatched = vi.fn();
    const nextToml =
      '[model_providers.third_party]\nbase_url = "https://api.example.test/v1"\nsupports_websockets = true\n';
    const patchedToml =
      '[model_providers.third_party]\nbase_url = "https://api.example.test/v1"\n';
    apiMocks.analyzeCodexProviderFeatures.mockResolvedValue(
      featureState({
        websockets: {
          enabled: true,
          compatible: false,
          reason: "CODEX_FEATURE_INCOMPATIBLE_WEBSOCKET",
        },
      }),
    );
    apiMocks.patchCodexProviderFeatures.mockResolvedValue({
      tomlText: patchedToml,
      state: featureState({
        websockets: {
          enabled: false,
          compatible: false,
          reason: "CODEX_FEATURE_INCOMPATIBLE_WEBSOCKET",
        },
      }),
    });

    const { result } = renderHook(() =>
      useCodexProviderFeatures({
        enabled: true,
        isNew: false,
        analysisKey: "initial",
        configText: nextToml,
        getDraft: () => draftFor(nextToml),
        onTomlPatched,
      }),
    );

    await act(async () => {
      await result.current.handleApiFormatChange("openai_chat", nextToml);
    });

    expect(apiMocks.analyzeCodexProviderFeatures).toHaveBeenCalledWith(
      expect.objectContaining({
        meta: expect.objectContaining({ apiFormat: "openai_chat" }),
        settingsConfig: expect.objectContaining({ config: nextToml }),
      }),
      false,
    );
    expect(apiMocks.patchCodexProviderFeatures).toHaveBeenCalledWith(
      expect.objectContaining({
        meta: expect.objectContaining({ apiFormat: "openai_chat" }),
      }),
      { websockets: false },
      false,
    );
    expect(onTomlPatched).toHaveBeenCalledWith(patchedToml);
    expect(result.current.websocketAutoDisabled).toBe(true);
  });

  it("blocks a manually restored incompatible WebSocket field instead of silently patching it", async () => {
    const tomlText =
      '[model_providers.third_party]\nbase_url = "https://api.example.test/v1"\nsupports_websockets = true\n';
    apiMocks.analyzeCodexProviderFeatures.mockResolvedValue(
      featureState({
        websockets: {
          enabled: true,
          compatible: false,
          reason: "CODEX_FEATURE_INCOMPATIBLE_WEBSOCKET",
        },
      }),
    );

    const { result } = renderHook(() =>
      useCodexProviderFeatures({
        enabled: true,
        isNew: false,
        analysisKey: "manual-incompatible-websocket",
        configText: tomlText,
        getDraft: () => draftFor(tomlText, "openai_chat"),
        onTomlPatched: () => undefined,
      }),
    );

    await expect(result.current.prepareForSave()).rejects.toThrow(
      "CODEX_FEATURE_INCOMPATIBLE_WEBSOCKET",
    );
    expect(apiMocks.patchCodexProviderFeatures).not.toHaveBeenCalled();
  });

  it("keeps an image-toggle migration marker in later form-local analysis and save preparation", async () => {
    const originalToml =
      '[model_providers.third_party]\nbase_url = "https://api.example.test/v1"\n';
    const patchedToml =
      '[model_providers.third_party]\nbase_url = "https://api.example.test/v1"\n';
    const onTomlPatched = vi.fn();
    apiMocks.patchCodexProviderFeatures.mockResolvedValue({
      tomlText: patchedToml,
      state: featureState({ imageExtension: { kind: "off" } }),
      imageExtensionConfigured: true,
    });
    apiMocks.analyzeCodexProviderFeatures.mockResolvedValue(
      featureState({ imageExtension: { kind: "off" } }),
    );

    const { result, rerender } = renderHook(
      ({ analysisKey, configText }) =>
        useCodexProviderFeatures({
          enabled: true,
          isNew: false,
          analysisKey,
          configText,
          getDraft: () => draftFor(configText),
          onTomlPatched,
        }),
      {
        initialProps: {
          analysisKey: "before-image-toggle",
          configText: originalToml,
        },
      },
    );

    await act(async () => {
      await result.current.patchFeatures({ imageExtension: false });
    });
    expect(onTomlPatched).toHaveBeenCalledWith(patchedToml);

    rerender({
      analysisKey: "after-image-toggle",
      configText: patchedToml,
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(250);
    });

    expect(apiMocks.analyzeCodexProviderFeatures).toHaveBeenLastCalledWith(
      expect.objectContaining({
        meta: expect.objectContaining({ imageExtensionConfigured: true }),
      }),
      false,
    );
    expect(result.current.state?.imageExtension).toEqual({ kind: "off" });

    await expect(result.current.prepareForSave()).resolves.toEqual({
      tomlText: patchedToml,
      imageExtensionConfigured: true,
    });
  });

  it("does not carry an unsaved image marker into an inapplicable provider", async () => {
    const tomlText =
      '[model_providers.third_party]\nbase_url = "https://api.example.test/v1"\n';
    apiMocks.patchCodexProviderFeatures.mockResolvedValue({
      tomlText,
      state: featureState({ imageExtension: { kind: "off" } }),
      imageExtensionConfigured: true,
    });
    apiMocks.analyzeCodexProviderFeatures.mockResolvedValue(
      featureState({
        applicable: false,
        imageExtension: { kind: "off" },
        providerTableFound: false,
      }),
    );

    const { result } = renderHook(() =>
      useCodexProviderFeatures({
        enabled: true,
        isNew: false,
        analysisKey: "became-official-or-incomplete",
        configText: tomlText,
        getDraft: () => draftFor(tomlText),
        onTomlPatched: () => undefined,
      }),
    );

    await act(async () => {
      await result.current.patchFeatures({ imageExtension: false });
    });

    await expect(result.current.prepareForSave()).resolves.toEqual({
      tomlText,
    });
  });

  it("suppresses a stale TOML analysis response after a later manual edit", async () => {
    let resolveFirst: ((state: CodexProviderFeatureState) => void) | undefined;
    let resolveSecond: ((state: CodexProviderFeatureState) => void) | undefined;
    apiMocks.analyzeCodexProviderFeatures
      .mockImplementationOnce(
        () =>
          new Promise<CodexProviderFeatureState>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise<CodexProviderFeatureState>((resolve) => {
            resolveSecond = resolve;
          }),
      );

    const { result, rerender } = renderHook(
      ({ analysisKey, configText }) =>
        useCodexProviderFeatures({
          enabled: true,
          isNew: false,
          analysisKey,
          configText,
          getDraft: () => draftFor(configText),
          onTomlPatched: () => undefined,
        }),
      {
        initialProps: {
          analysisKey: "first",
          configText: "[model_providers.first]",
        },
      },
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(250);
    });
    expect(resolveFirst).toBeTypeOf("function");

    rerender({
      analysisKey: "second",
      configText: "[model_providers.second]",
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(250);
    });
    expect(resolveSecond).toBeTypeOf("function");

    await act(async () => {
      resolveSecond?.(featureState({ providerTableFound: true }));
    });
    expect(result.current.state?.providerTableFound).toBe(true);

    await act(async () => {
      resolveFirst?.(featureState({ providerTableFound: false }));
    });
    expect(result.current.state?.providerTableFound).toBe(true);
  });
});
