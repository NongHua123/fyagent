import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps, PropsWithChildren } from "react";
import { useForm } from "react-hook-form";
import { describe, expect, it, vi } from "vitest";
import { CodexFormFields } from "@/components/providers/forms/CodexFormFields";
import { Form } from "@/components/ui/form";
import type { CodexProviderFeatureState } from "@/lib/api";
import type { ProviderCategory } from "@/types";

vi.mock("@/components/providers/forms/XaiOAuthSection", () => ({
  XaiOAuthSection: () => <div data-testid="xai-oauth-section" />,
}));

type CodexFormFieldsProps = ComponentProps<typeof CodexFormFields>;

const validFeatureState: CodexProviderFeatureState = {
  applicable: true,
  imageExtension: { kind: "off" },
  websockets: { enabled: false, compatible: true },
  providerTableFound: false,
  diagnostics: [],
};

const FormShell = ({ children }: PropsWithChildren) => {
  const form = useForm();
  return <Form {...form}>{children}</Form>;
};

function renderCodexFields(overrides: Partial<CodexFormFieldsProps> = {}) {
  const props: CodexFormFieldsProps = {
    appId: "codex",
    providerId: "provider-1",
    codexApiKey: "test-only",
    onApiKeyChange: vi.fn(),
    category: "custom",
    shouldShowApiKeyLink: false,
    websiteUrl: "",
    shouldShowSpeedTest: true,
    codexBaseUrl: "https://api.example.test/v1",
    onBaseUrlChange: vi.fn(),
    isFullUrl: false,
    onFullUrlChange: vi.fn(),
    isEndpointModalOpen: false,
    onEndpointModalToggle: vi.fn(),
    onCustomEndpointsChange: vi.fn(),
    autoSelect: false,
    onAutoSelectChange: vi.fn(),
    codexModel: "gpt-5.6-sol",
    onModelChange: vi.fn(),
    apiFormat: "openai_responses",
    onApiFormatChange: vi.fn(),
    codexFeatureState: validFeatureState,
    isCodexFeatureAnalyzing: false,
    isCodexFeaturePatching: false,
    codexFeatureError: null,
    onCodexImageExtensionChange: vi.fn(),
    onCodexWebsocketsChange: vi.fn(),
    anthropicAuthField: "ANTHROPIC_AUTH_TOKEN",
    onAnthropicAuthFieldChange: vi.fn(),
    impersonateClaudeCode: false,
    onImpersonateClaudeCodeChange: vi.fn(),
    maxOutputTokens: "",
    onMaxOutputTokensChange: vi.fn(),
    codexChatReasoning: {},
    onCodexChatReasoningChange: vi.fn(),
    promptCacheRouting: "auto",
    onPromptCacheRoutingChange: vi.fn(),
    catalogModels: [],
    onCatalogModelsChange: vi.fn(),
    speedTestEndpoints: [],
    customUserAgent: "",
    onCustomUserAgentChange: vi.fn(),
    localProxyHeadersOverride: "",
    onLocalProxyHeadersOverrideChange: vi.fn(),
    localProxyBodyOverride: "",
    onLocalProxyBodyOverrideChange: vi.fn(),
    ...overrides,
  };

  return render(
    <FormShell>
      <CodexFormFields {...props} />
    </FormShell>,
  );
}

describe("CodexFormFields native capabilities", () => {
  it.each<ProviderCategory>([
    "official",
    "custom",
    "cloud_provider",
    "third_party",
  ])(
    "shows both switches for %s providers inside collapsed advanced options",
    (category) => {
      const view = renderCodexFields({ category });

      expect(
        screen.queryByRole("switch", { name: "启用内置生图扩展" }),
      ).not.toBeInTheDocument();
      fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
      expect(
        screen.getByRole("switch", { name: "启用内置生图扩展" }),
      ).toBeEnabled();
      expect(
        screen.getByRole("switch", { name: "启用 WebSocket 传输" }),
      ).toBeEnabled();

      view.unmount();
    },
  );

  it("treats the fixed official id as official and exposes no unrelated advanced fields", () => {
    renderCodexFields({
      providerId: "codex-official",
      category: "custom",
    });

    fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
    expect(
      screen.getByRole("switch", { name: "启用内置生图扩展" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("switch", { name: "启用 WebSocket 传输" }),
    ).toBeEnabled();
    expect(screen.queryByText("上游格式")).not.toBeInTheDocument();
    expect(screen.queryByText("模型映射")).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/User-Agent/)).not.toBeInTheDocument();
  });

  it("keeps both controls available for a managed xAI OAuth preset", () => {
    renderCodexFields({
      providerId: "xai-managed-provider",
      category: "custom",
      isXaiOauthPreset: true,
      shouldShowSpeedTest: false,
    });

    fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
    expect(
      screen.getByRole("switch", { name: "启用内置生图扩展" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("switch", { name: "启用 WebSocket 传输" }),
    ).toBeEnabled();
  });
});
