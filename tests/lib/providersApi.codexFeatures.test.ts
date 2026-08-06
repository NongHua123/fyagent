import { beforeEach, describe, expect, it, vi } from "vitest";
import { providersApi } from "@/lib/api/providers";
import type { Provider } from "@/types";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const draft: Provider = {
  id: "provider-1",
  name: "Third-party provider",
  category: "custom",
  settingsConfig: {
    auth: { OPENAI_API_KEY: "test-only" },
    config: 'model_provider = "third-party"\n',
  },
  meta: { apiFormat: "openai_responses" },
};

beforeEach(() => {
  invokeMock.mockReset().mockResolvedValue({});
});

describe("Codex native capability IPC wrappers", () => {
  it("pins feature analysis to the Codex provider domain", async () => {
    await providersApi.analyzeCodexProviderFeatures(draft, true);

    expect(invokeMock).toHaveBeenCalledWith("analyze_codex_provider_features", {
      app: "codex",
      provider: draft,
      isNew: true,
    });
  });

  it("pins draft patches to Codex and sends only the requested intent", async () => {
    await providersApi.patchCodexProviderFeatures(
      draft,
      { websockets: false },
      false,
    );

    expect(invokeMock).toHaveBeenCalledWith("patch_codex_provider_features", {
      app: "codex",
      provider: draft,
      intent: { websockets: false },
      isNew: false,
    });
  });

  it("uses the result-aware delete command so restart prompting follows the backend byte delta", async () => {
    const expected = {
      value: true,
      liveConfigChanged: false,
      app: "codex",
    };
    invokeMock.mockResolvedValueOnce(expected);

    await expect(
      providersApi.deleteWithResult("provider-1", "codex"),
    ).resolves.toEqual(expected);

    expect(invokeMock).toHaveBeenCalledWith("delete_provider_with_result", {
      id: "provider-1",
      app: "codex",
    });
  });
});
