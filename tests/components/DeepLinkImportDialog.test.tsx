import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { http, HttpResponse, type DefaultBodyType } from "msw";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DeepLinkImportDialog } from "@/components/DeepLinkImportDialog";
import { server } from "../msw/server";
import { emitTauriEvent } from "../msw/tauriMocks";

const mocks = vi.hoisted(() => ({
  toastError: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => mocks.toastError(...args),
    success: vi.fn(),
    warning: vi.fn(),
  },
}));

function renderDialog() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <DeepLinkImportDialog />
    </QueryClientProvider>,
  );
}

describe("DeepLinkImportDialog", () => {
  beforeEach(() => {
    mocks.toastError.mockReset();
  });

  it("does not expose a rejected deep-link URL or API key in renderer errors", () => {
    const apiKey = "sk-deeplink-renderer-secret";
    const rejectedUrl = `fyagent://v1/import?resource=provider&apiKey=${apiKey}`;
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    renderDialog();

    act(() => {
      emitTauriEvent("deeplink-error", {
        url: rejectedUrl,
        error: `Rejected ${rejectedUrl}`,
      });
    });

    expect(mocks.toastError).toHaveBeenCalledWith("deeplink.parseError");
    expect(JSON.stringify(mocks.toastError.mock.calls)).not.toContain(apiKey);
    expect(JSON.stringify(mocks.toastError.mock.calls)).not.toContain(
      rejectedUrl,
    );
    expect(consoleError).not.toHaveBeenCalled();

    consoleError.mockRestore();
  });

  it("renders the entire prompt payload that confirmation can write", async () => {
    const promptContent = `${"reviewable-prompt-content-".repeat(48)}tail-must-remain-visible`;

    renderDialog();

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "prompt",
        app: "claude",
        name: "Review every byte",
        content: Buffer.from(promptContent, "utf8").toString("base64"),
        enabled: true,
      });
    });

    expect(await screen.findByText(promptContent)).toBeInTheDocument();
    expect(
      screen.getByText("deeplink.prompt.enabledWarning"),
    ).toBeInTheDocument();
  });

  it("submits provider activation approval only after a separate user choice", async () => {
    const submittedRequests: Array<Record<string, unknown>> = [];
    server.use(
      http.post(
        "http://tauri.local/import_from_deeplink_unified",
        async ({ request }) => {
          const body = (await request.json()) as {
            request?: Record<string, unknown>;
          };
          submittedRequests.push(body.request ?? {});
          return HttpResponse.json({
            type: "provider",
            id: `provider-${submittedRequests.length}`,
          });
        },
      ),
    );

    renderDialog();

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "Review before activation",
        endpoint: "https://api.example.test/v1",
        apiKey: "sk-confirmation-test-key",
        enabled: true,
      });
    });

    expect(
      await screen.findByText("deeplink.providerActivationPending"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("deeplink.providerActivationWarning"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "deeplink.import" }));
    await waitFor(() => expect(submittedRequests).toHaveLength(1));
    expect(submittedRequests[0]).toMatchObject({
      enabled: true,
      activationApproved: false,
    });

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "Review before activation",
        endpoint: "https://api.example.test/v1",
        apiKey: "sk-confirmation-test-key",
        enabled: true,
      });
    });

    expect(
      await screen.findByText("deeplink.providerActivationPending"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox"));

    expect(
      screen.getByText("deeplink.providerActivationEnabled"),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", {
        name: "deeplink.importAndActivate",
      }),
    );
    await waitFor(() => expect(submittedRequests).toHaveLength(2));
    expect(submittedRequests[1]).toMatchObject({
      enabled: true,
      activationApproved: true,
    });
  });

  it("does not carry a later activation approval into an earlier config merge", async () => {
    let resolveFirstMerge:
      | ((response: HttpResponse<DefaultBodyType>) => void)
      | undefined;
    let firstMergeStarted = false;
    const submittedRequests: Array<Record<string, unknown>> = [];

    server.use(
      http.post(
        "http://tauri.local/merge_deeplink_config",
        async ({ request }) => {
          const body = (await request.json()) as {
            request?: Record<string, unknown>;
          };
          const name = body.request?.name;
          if (name !== "Earlier config link") {
            return HttpResponse.json(body.request);
          }

          firstMergeStarted = true;
          return new Promise<HttpResponse<DefaultBodyType>>((resolve) => {
            resolveFirstMerge = resolve;
          });
        },
      ),
      http.post(
        "http://tauri.local/import_from_deeplink_unified",
        async ({ request }) => {
          const body = (await request.json()) as {
            request?: Record<string, unknown>;
          };
          submittedRequests.push(body.request ?? {});
          return HttpResponse.json({ type: "provider", id: "later-provider" });
        },
      ),
    );

    renderDialog();

    act(() => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "Earlier config link",
        endpoint: "https://earlier.example.test/v1",
        apiKey: "sk-earlier-test-key",
        enabled: true,
        config: "pending-config",
      });
    });
    await waitFor(() => expect(firstMergeStarted).toBe(true));

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "Later approval link",
        endpoint: "https://later.example.test/v1",
        apiKey: "sk-later-test-key",
        enabled: true,
      });
    });

    expect(await screen.findByText("Later approval link")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox"));
    expect(
      screen.getByText("deeplink.providerActivationEnabled"),
    ).toBeInTheDocument();

    expect(resolveFirstMerge).toBeDefined();
    await act(async () => {
      resolveFirstMerge?.(
        HttpResponse.json({
          version: "v1",
          resource: "provider",
          app: "codex",
          name: "Earlier config link",
          endpoint: "https://earlier.example.test/v1",
          apiKey: "sk-earlier-test-key",
          enabled: true,
        }),
      );
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByText("Later approval link")).toBeInTheDocument();
    expect(screen.queryByText("Earlier config link")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "deeplink.importAndActivate" }),
    );
    await waitFor(() => expect(submittedRequests).toHaveLength(1));
    expect(submittedRequests[0]).toMatchObject({
      name: "Later approval link",
      enabled: true,
      activationApproved: true,
    });
  });

  it("keeps a newer deep link open when an older import finishes", async () => {
    let resolveFirstImport:
      | ((response: HttpResponse<DefaultBodyType>) => void)
      | undefined;
    let firstImportStarted = false;

    server.use(
      http.post(
        "http://tauri.local/import_from_deeplink_unified",
        async ({ request }) => {
          const body = (await request.json()) as {
            request?: Record<string, unknown>;
          };
          if (body.request?.name !== "First import") {
            return HttpResponse.json({ type: "provider", id: "second" });
          }

          firstImportStarted = true;
          return new Promise<HttpResponse<DefaultBodyType>>((resolve) => {
            resolveFirstImport = resolve;
          });
        },
      ),
    );

    renderDialog();

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "First import",
        endpoint: "https://first.example.test/v1",
        apiKey: "sk-first-test-key",
      });
    });
    expect(await screen.findByText("First import")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "deeplink.import" }));
    await waitFor(() => expect(firstImportStarted).toBe(true));

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "Second import",
        endpoint: "https://second.example.test/v1",
        apiKey: "sk-second-test-key",
      });
    });
    expect(await screen.findByText("Second import")).toBeInTheDocument();

    expect(resolveFirstImport).toBeDefined();
    await act(async () => {
      resolveFirstImport?.(
        HttpResponse.json({ type: "provider", id: "first" }),
      );
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByText("Second import")).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "deeplink.import" }),
      ).not.toBeDisabled(),
    );
  });

  it("states when a provider import will not switch the current provider", async () => {
    renderDialog();

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "codex",
        name: "Import only",
        endpoint: "https://api.example.test/v1",
        apiKey: "sk-confirmation-test-key",
      });
    });

    expect(
      await screen.findByText("deeplink.providerActivationDisabled"),
    ).toBeInTheDocument();
  });

  it("renders masked usage access token and user id for provider imports", async () => {
    renderDialog();

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "claude",
        name: "Test Provider",
        homepage: "https://example.com",
        endpoint: "https://api.example.com",
        apiKey: "sk-provider-key",
        usageEnabled: true,
        usageScript: btoa("console.log('usage');"),
        usageApiKey: "sk-usage-key",
        usageBaseUrl: "https://usage.example.com",
        usageAccessToken: "pat-secret-token",
        usageUserId: "user-12345",
        usageAutoInterval: 60,
      });
    });

    expect(await screen.findByText("用量访问令牌")).toBeInTheDocument();
    expect(screen.getByText("用量用户 ID")).toBeInTheDocument();
    expect(screen.getByText("user-12345")).toBeInTheDocument();
    expect(screen.getByText("pat-************")).toBeInTheDocument();
  });

  it("shows usage credentials even when the deep link carries no usage script", async () => {
    renderDialog();

    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "v1",
        resource: "provider",
        app: "claude",
        name: "Token Only Provider",
        homepage: "https://example.com",
        endpoint: "https://api.example.com",
        apiKey: "sk-provider-key",
        usageAccessToken: "pat-secret-token",
        usageUserId: "user-12345",
      });
    });

    expect(await screen.findByText("用量访问令牌")).toBeInTheDocument();
    expect(screen.getByText("pat-************")).toBeInTheDocument();
    expect(screen.getByText("用量用户 ID")).toBeInTheDocument();
    expect(screen.getByText("user-12345")).toBeInTheDocument();
    expect(
      screen.queryByText(
        "这是一段 JavaScript 代码，启用后会在查询用量时执行。请确认来源可信后再导入。",
      ),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("脚本代码")).not.toBeInTheDocument();
  });
});
