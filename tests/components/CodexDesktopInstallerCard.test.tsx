import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CodexDesktopInstallerCard } from "@/components/codex/CodexDesktopInstallerCard";
import type { CodexDesktopInstallerViewModel } from "@/hooks/useCodexDesktopInstaller";

const mocks = vi.hoisted(() => ({
  useInstaller: vi.fn(),
}));

vi.mock("@/hooks/useCodexDesktopInstaller", () => ({
  useCodexDesktopInstaller: () => mocks.useInstaller(),
}));

function createViewModel(
  overrides: Partial<CodexDesktopInstallerViewModel> = {},
): CodexDesktopInstallerViewModel {
  return {
    state: "ready_install",
    localVersion: undefined,
    remoteVersion: "26.1.0",
    progress: undefined,
    primaryAction: "install",
    primaryDisabled: false,
    canCancel: false,
    error: null,
    isRefreshing: false,
    refresh: vi.fn().mockResolvedValue(undefined),
    runPrimaryAction: vi.fn().mockResolvedValue(undefined),
    cancel: vi.fn().mockResolvedValue(undefined),
    copyErrorDetails: vi.fn().mockResolvedValue(undefined),
    openLogs: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

beforeEach(() => {
  mocks.useInstaller.mockReset();
});

describe("CodexDesktopInstallerCard", () => {
  it("offers Update Codex as the only action when a newer release is available", () => {
    const installer = createViewModel({
      state: "ready_update",
      localVersion: "26.0.0",
      primaryAction: "update",
    });
    mocks.useInstaller.mockReturnValue(installer);

    render(<CodexDesktopInstallerCard />);

    const updateButton = screen.getByRole("button", {
      name: "codexDesktop.actions.update",
    });
    expect(updateButton).toBeVisible();
    expect(
      screen.queryByRole("button", {
        name: "codexDesktop.actions.launch",
      }),
    ).not.toBeInTheDocument();

    fireEvent.click(updateButton);
    expect(installer.runPrimaryAction).toHaveBeenCalledOnce();
  });

  it("exposes bounded download progress and only renders cancellation while the backend allows it", () => {
    const installer = createViewModel({
      state: "job_downloading",
      primaryAction: null,
      primaryDisabled: true,
      canCancel: true,
      progress: { current: 512, total: 1024, percent: 50 },
    });
    mocks.useInstaller.mockReturnValue(installer);

    render(<CodexDesktopInstallerCard />);

    expect(
      screen.getByRole("progressbar", {
        name: "codexDesktop.details.progress",
      }),
    ).toHaveAttribute("aria-valuenow", "50");
    expect(screen.getByText("50% · 512 B / 1 KB")).toBeVisible();
    const cancelButton = screen.getByRole("button", {
      name: "codexDesktop.actions.cancel",
    });
    fireEvent.click(cancelButton);
    expect(installer.cancel).toHaveBeenCalledOnce();
  });

  it("shows percentage without completed download bytes while installing", () => {
    mocks.useInstaller.mockReturnValue(
      createViewModel({
        state: "job_installing",
        primaryAction: null,
        primaryDisabled: true,
        progress: { current: 1024, total: 1024, percent: 100 },
      }),
    );

    render(<CodexDesktopInstallerCard />);

    expect(screen.getByText("100%")).toBeVisible();
    expect(screen.queryByText(/1 KB/)).not.toBeInTheDocument();
  });

  it.each([
    ["ready_launch", "26.1.0"],
    ["local_newer", "26.2.0"],
    ["remote_unavailable_installed", "26.1.0"],
  ] as const)(
    "offers Launch Codex for the %s installed state",
    (state, localVersion) => {
      const installer = createViewModel({
        state,
        localVersion,
        remoteVersion:
          state === "remote_unavailable_installed" ? undefined : "26.1.0",
        primaryAction: "launch",
      });
      mocks.useInstaller.mockReturnValue(installer);

      render(<CodexDesktopInstallerCard />);

      const launchButton = screen.getByRole("button", {
        name: "codexDesktop.actions.launch",
      });
      fireEvent.click(launchButton);
      expect(installer.runPrimaryAction).toHaveBeenCalledOnce();
    },
  );

  it.each(["job_installing", "job_verifying_installation"] as const)(
    "does not offer cancellation during %s",
    (state) => {
      mocks.useInstaller.mockReturnValue(
        createViewModel({
          state,
          primaryAction: null,
          primaryDisabled: true,
          canCancel: false,
        }),
      );

      render(<CodexDesktopInstallerCard />);

      expect(
        screen.queryByRole("button", {
          name: "codexDesktop.actions.cancel",
        }),
      ).not.toBeInTheDocument();
    },
  );

  it("renders no UI when Linux is reported as an unsupported platform", () => {
    mocks.useInstaller.mockReturnValue(
      createViewModel({
        state: "hidden",
        primaryAction: null,
        primaryDisabled: true,
      }),
    );

    const { container } = render(<CodexDesktopInstallerCard />);

    expect(container).toBeEmptyDOMElement();
  });

  it("renders the unsupported state without actions for an Intel Mac", () => {
    mocks.useInstaller.mockReturnValue(
      createViewModel({
        state: "unsupported_architecture",
        remoteVersion: undefined,
        primaryAction: null,
        primaryDisabled: true,
      }),
    );

    render(<CodexDesktopInstallerCard />);

    expect(screen.getByText("codexDesktop.title")).toBeVisible();
    expect(screen.getByText("unsupported_architecture")).toBeVisible();
    expect(
      screen.queryByRole("button", {
        name: /codexDesktop\.actions\.(install|update|launch|retry)/,
      }),
    ).not.toBeInTheDocument();
  });

  it("shows only backend-redacted diagnostics and delegates copying", () => {
    const installer = createViewModel({
      state: "failed",
      primaryAction: "retry",
      error: {
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
          redactedMessage:
            "GET https://mirror.example/releases?[REDACTED] failed",
          context: { source: "agentsmirror" },
        },
      },
    });
    mocks.useInstaller.mockReturnValue(installer);

    render(<CodexDesktopInstallerCard />);

    expect(screen.getAllByText(/\[REDACTED\]/)).not.toHaveLength(0);
    expect(screen.getByText(/DOWNLOAD_FAILED/)).toBeInTheDocument();
    expect(
      screen.queryByText(/token=unredacted-secret/),
    ).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: "codexDesktop.actions.copyErrorDetails",
      }),
    );
    expect(installer.copyErrorDetails).toHaveBeenCalledOnce();
  });

  it("keeps source as read-only provenance and exposes no installer configuration controls", () => {
    mocks.useInstaller.mockReturnValue(createViewModel());

    const { container } = render(<CodexDesktopInstallerCard />);

    expect(screen.getByText("codexDesktop.source")).toBeVisible();
    expect(container.querySelectorAll("input, select, textarea")).toHaveLength(
      0,
    );
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(screen.queryByRole("radio")).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(screen.getAllByRole("button")).toHaveLength(2);
    expect(
      screen.getByRole("button", { name: "codexDesktop.actions.refresh" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "codexDesktop.actions.install" }),
    ).toBeVisible();
  });
});
