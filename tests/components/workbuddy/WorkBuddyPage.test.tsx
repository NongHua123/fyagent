import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import i18n from "i18next";
import { http, HttpResponse, type DefaultBodyType } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import { WorkBuddyPage } from "@/components/workbuddy/WorkBuddyPage";
import { server } from "../../msw/server";

const TAURI_ENDPOINT = "http://tauri.local";

const renderWorkBuddyPage = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <WorkBuddyPage />
    </QueryClientProvider>,
  );
};

afterEach(() => {
  vi.restoreAllMocks();
  clearWorkBuddyErrorTranslations();
});

const addWorkBuddyErrorTranslations = () => {
  i18n.addResourceBundle(
    "zh",
    "translation",
    {
      workbuddy: {
        error: {
          fetchHttp: "The model service returned an HTTP error.",
          configInvalidEntry: "A model entry is invalid.",
          internal: "The WorkBuddy request could not be completed.",
          httpStatus: "HTTP status: {{status}}",
          redactedSummary: "Upstream summary: {{summary}}",
          invalidEntryIndex: "Invalid configuration item at index {{index}}",
        },
      },
    },
    true,
    true,
  );
};

const clearWorkBuddyErrorTranslations = () => {
  i18n.removeResourceBundle("zh", "translation");
  i18n.addResourceBundle("zh", "translation", {});
};

describe("WorkBuddyPage", () => {
  it("keeps the remote HTTP warning visible without blocking fetch or save", async () => {
    const fetchRequest = vi.fn();
    const saveRequest = vi.fn();

    server.use(
      http.post(`${TAURI_ENDPOINT}/get_workbuddy_status`, () =>
        HttpResponse.json({
          path: "C:/Users/test/.workbuddy/models.json",
          exists: true,
          modelCount: 1,
          revision: "revision-1",
          backupExists: false,
        }),
      ),
      http.post(
        `${TAURI_ENDPOINT}/fetch_workbuddy_models`,
        async ({ request }) => {
          fetchRequest(await request.json());
          return HttpResponse.json({ models: ["gpt-test"], truncated: false });
        },
      ),
      http.post(
        `${TAURI_ENDPOINT}/save_workbuddy_models`,
        async ({ request }) => {
          saveRequest(await request.json());
          return HttpResponse.json(true);
        },
      ),
    );

    renderWorkBuddyPage();

    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "http://192.168.50.10:8080/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "test-api-key" },
    });

    expect(screen.getByText("workbuddy.httpWarning.title")).toBeVisible();
    expect(screen.getByText("workbuddy.httpWarning.description")).toBeVisible();

    fireEvent.click(
      screen.getByRole("button", { name: "workbuddy.fetchModels" }),
    );

    await waitFor(() => expect(fetchRequest).toHaveBeenCalledTimes(1));
    expect(screen.getByText("workbuddy.httpWarning.title")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "workbuddy.save" }));

    await waitFor(() => expect(saveRequest).toHaveBeenCalledTimes(1));
    expect(saveRequest).toHaveBeenCalledWith(
      expect.objectContaining({
        request: expect.objectContaining({
          baseUrl: "http://192.168.50.10:8080/v1",
          apiKey: "test-api-key",
          selectedModelIds: ["gpt-test"],
          duplicatePolicy: "reject",
        }),
      }),
    );
    expect(screen.getByText("workbuddy.httpWarning.title")).toBeVisible();
  });

  it("keeps the truncation warning after model selection or manual edits", async () => {
    server.use(
      http.post(`${TAURI_ENDPOINT}/get_workbuddy_status`, () =>
        HttpResponse.json({
          path: "C:/Users/test/.workbuddy/models.json",
          exists: true,
          modelCount: 0,
          revision: "revision-1",
          backupExists: false,
        }),
      ),
      http.post(`${TAURI_ENDPOINT}/fetch_workbuddy_models`, () =>
        HttpResponse.json({
          models: ["gpt-one", "gpt-two"],
          truncated: true,
        }),
      ),
    );

    renderWorkBuddyPage();
    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "https://api.example.test/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "test-api-key" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "workbuddy.fetchModels" }),
    );

    await screen.findByText("workbuddy.models.truncatedTitle");
    fireEvent.click(
      screen.getByRole("button", { name: "workbuddy.models.deselectAll" }),
    );
    fireEvent.change(screen.getByLabelText("workbuddy.models.manual"), {
      target: { value: "manual-model" },
    });

    expect(screen.getByText("workbuddy.models.truncatedTitle")).toBeVisible();
    expect(
      screen.getByText("workbuddy.models.truncatedDescription"),
    ).toBeVisible();
  });

  it("freezes the duplicate-conflict request and retries only with updateAll", async () => {
    const requests: Array<{ request: Record<string, unknown> }> = [];

    server.use(
      http.post(`${TAURI_ENDPOINT}/get_workbuddy_status`, () =>
        HttpResponse.json({
          path: "C:/Users/test/.workbuddy/models.json",
          exists: true,
          modelCount: 1,
          revision: "revision-1",
          backupExists: false,
        }),
      ),
      http.post(
        `${TAURI_ENDPOINT}/save_workbuddy_models`,
        async ({ request }) => {
          const body = (await request.json()) as {
            request: Record<string, unknown>;
          };
          requests.push(body);
          if (requests.length === 1) {
            return HttpResponse.json(
              {
                code: "WORKBUDDY_CONFIG_DUPLICATE_TARGET",
                messageKey: "workbuddy.error.configDuplicateTarget",
                details: {
                  duplicateIds: [{ id: "duplicate-model", count: 2 }],
                },
              },
              { status: 409 },
            );
          }
          return HttpResponse.json({
            revision: "revision-2",
            modelCount: 1,
            createdEntries: 0,
            updatedEntries: 2,
            duplicateIds: [{ id: "duplicate-model", count: 2 }],
          });
        },
      ),
    );

    renderWorkBuddyPage();
    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "https://first.example.test/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "first-key" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.models.manual"), {
      target: { value: "duplicate-model" },
    });
    fireEvent.click(screen.getByRole("button", { name: "workbuddy.save" }));

    await screen.findByText("workbuddy.duplicateDialog.title");
    expect(requests).toHaveLength(1);

    // A user may continue editing while the conflict prompt is open. The
    // confirmation must not silently save those later values.
    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "https://later.example.test/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "later-key" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.models.manual"), {
      target: { value: "later-model" },
    });
    fireEvent.click(
      screen.getByRole("button", {
        name: "workbuddy.duplicateDialog.confirm",
      }),
    );

    await waitFor(() => expect(requests).toHaveLength(2));
    expect(requests[1]?.request).toEqual({
      ...requests[0]?.request,
      duplicatePolicy: "updateAll",
    });
  });

  it("does not retain an API key when its page is unmounted", async () => {
    let resolveFetch:
      | ((response: HttpResponse<DefaultBodyType>) => void)
      | undefined;
    const fetchStarted = vi.fn();

    server.use(
      http.post(`${TAURI_ENDPOINT}/get_workbuddy_status`, () =>
        HttpResponse.json({
          path: "C:/Users/test/.workbuddy/models.json",
          exists: false,
          modelCount: 0,
          revision: null,
          backupExists: false,
        }),
      ),
      http.post(`${TAURI_ENDPOINT}/fetch_workbuddy_models`, () => {
        fetchStarted();
        return new Promise<HttpResponse<DefaultBodyType>>((resolve) => {
          resolveFetch = resolve;
        });
      }),
    );

    const first = renderWorkBuddyPage();
    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "https://api.example.test/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "sensitive-test-key" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "workbuddy.fetchModels" }),
    );
    await waitFor(() => expect(fetchStarted).toHaveBeenCalledTimes(1));

    first.unmount();
    resolveFetch?.(
      HttpResponse.json({ models: ["late-model"], truncated: false }),
    );

    renderWorkBuddyPage();
    expect(screen.getByLabelText("workbuddy.apiKey")).toHaveValue("");
    expect(screen.queryByText("late-model")).not.toBeInTheDocument();
  });

  it("renders only the structured, redacted WorkBuddy error details", async () => {
    addWorkBuddyErrorTranslations();
    server.use(
      http.post(`${TAURI_ENDPOINT}/get_workbuddy_status`, () =>
        HttpResponse.json({
          path: "C:/Users/test/.workbuddy/models.json",
          exists: true,
          modelCount: 1,
          revision: "revision-1",
          backupExists: false,
        }),
      ),
      http.post(`${TAURI_ENDPOINT}/fetch_workbuddy_models`, () =>
        HttpResponse.json(
          {
            code: "WORKBUDDY_FETCH_HTTP_ERROR",
            messageKey: "workbuddy.error.fetchHttp",
            details: {
              httpStatus: 401,
              redactedSummary: "Authentication was rejected.",
            },
          },
          { status: 401 },
        ),
      ),
      http.post(`${TAURI_ENDPOINT}/save_workbuddy_models`, () =>
        HttpResponse.json(
          {
            code: "WORKBUDDY_CONFIG_INVALID_ENTRY",
            messageKey: "workbuddy.error.configInvalidEntry",
            details: { invalidEntryIndex: 2 },
          },
          { status: 400 },
        ),
      ),
    );

    renderWorkBuddyPage();
    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "https://api.example.test/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "sensitive-test-key" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "workbuddy.fetchModels" }),
    );

    const fetchErrorMessage = await screen.findByText(
      "The model service returned an HTTP error.",
    );
    expect(fetchErrorMessage.closest('[role="alert"]')).toBeTruthy();
    expect(screen.getByText("HTTP status: 401")).toBeVisible();
    expect(
      screen.getByText("Upstream summary: Authentication was rejected."),
    ).toBeVisible();
    expect(screen.queryByText("sensitive-test-key")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("workbuddy.models.manual"), {
      target: { value: "manual-model" },
    });
    fireEvent.click(screen.getByRole("button", { name: "workbuddy.save" }));

    await screen.findByText("A model entry is invalid.");
    expect(
      screen.getByText("Invalid configuration item at index 2"),
    ).toBeVisible();
  });

  it("keeps unstructured upstream failures generic", async () => {
    addWorkBuddyErrorTranslations();
    server.use(
      http.post(`${TAURI_ENDPOINT}/get_workbuddy_status`, () =>
        HttpResponse.json({
          path: "C:/Users/test/.workbuddy/models.json",
          exists: false,
          modelCount: 0,
          revision: null,
          backupExists: false,
        }),
      ),
      http.post(`${TAURI_ENDPOINT}/fetch_workbuddy_models`, () =>
        HttpResponse.text("unstructured failure: sensitive-test-key", {
          status: 500,
        }),
      ),
    );

    renderWorkBuddyPage();
    fireEvent.change(screen.getByLabelText("workbuddy.baseUrl"), {
      target: { value: "https://api.example.test/v1" },
    });
    fireEvent.change(screen.getByLabelText("workbuddy.apiKey"), {
      target: { value: "sensitive-test-key" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "workbuddy.fetchModels" }),
    );

    await screen.findByText("The WorkBuddy request could not be completed.");
    expect(
      screen.queryByText("unstructured failure: sensitive-test-key"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/HTTP status:/)).not.toBeInTheDocument();
  });
});
