import { fireEvent, render, screen } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { useForm } from "react-hook-form";
import { describe, expect, it, vi } from "vitest";
import { CodexNativeCapabilities } from "@/components/providers/forms/codex/CodexNativeCapabilities";
import { Form } from "@/components/ui/form";
import type { CodexProviderFeatureState } from "@/lib/api";

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

const FormShell = ({ children }: PropsWithChildren) => {
  const form = useForm();
  return <Form {...form}>{children}</Form>;
};

function renderCapabilities(
  state: CodexProviderFeatureState | null,
  handlers: {
    onImageExtensionChange?: (enabled: boolean) => void;
    onWebsocketsChange?: (enabled: boolean) => void;
  } = {},
) {
  return render(
    <FormShell>
      <CodexNativeCapabilities
        state={state}
        isAnalyzing={false}
        isPatching={false}
        error={null}
        onImageExtensionChange={handlers.onImageExtensionChange ?? vi.fn()}
        onWebsocketsChange={handlers.onWebsocketsChange ?? vi.fn()}
      />
    </FormShell>,
  );
}

describe("CodexNativeCapabilities", () => {
  it("always renders both editable capability switches for valid TOML", () => {
    const onImageExtensionChange = vi.fn();
    const onWebsocketsChange = vi.fn();
    renderCapabilities(featureState(), {
      onImageExtensionChange,
      onWebsocketsChange,
    });

    const imageSwitch = screen.getByRole("switch", {
      name: "启用内置生图扩展",
    });
    const websocketSwitch = screen.getByRole("switch", {
      name: "启用 WebSocket 传输",
    });
    expect(imageSwitch).toBeEnabled();
    expect(websocketSwitch).toBeEnabled();

    fireEvent.click(imageSwitch);
    fireEvent.click(websocketSwitch);
    expect(onImageExtensionChange).toHaveBeenCalledWith(true);
    expect(onWebsocketsChange).toHaveBeenCalledWith(true);
  });

  it("keeps switches visible but disables them for an invalid TOML document", () => {
    renderCapabilities(
      featureState({
        applicable: false,
        imageExtension: {
          kind: "invalid",
          code: "CODEX_FEATURE_INVALID_TOML",
        },
        websockets: { enabled: false, compatible: false },
        providerTableFound: false,
        diagnostics: [{ code: "CODEX_FEATURE_INVALID_TOML", field: "config" }],
      }),
    );

    expect(
      screen.getByRole("switch", { name: "启用内置生图扩展" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("switch", { name: "启用 WebSocket 传输" }),
    ).toBeDisabled();
    expect(screen.getAllByRole("alert").length).toBeGreaterThan(0);
  });

  it("leaves both switches editable when only capability field types are invalid", () => {
    renderCapabilities(
      featureState({
        imageExtension: {
          kind: "invalid",
          code: "CODEX_FEATURE_INVALID_HEADER",
        },
        diagnostics: [
          { code: "CODEX_FEATURE_INVALID_HEADER", field: "httpHeaders" },
          {
            code: "CODEX_FEATURE_INVALID_WEBSOCKET",
            field: "supportsWebsockets",
          },
        ],
      }),
    );

    expect(
      screen.getByRole("switch", { name: "启用内置生图扩展" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("switch", { name: "启用 WebSocket 传输" }),
    ).toBeEnabled();
  });
});
