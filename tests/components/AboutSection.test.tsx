import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AboutSection } from "@/components/settings/AboutSection";

const mocks = vi.hoisted(() => ({
  getVersion: vi.fn(),
  getToolVersions: vi.fn(),
  probeToolInstallations: vi.fn(),
  runToolLifecycleAction: vi.fn(),
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
  toastInfo: vi.fn(),
  toastWarning: vi.fn(),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: mocks.getVersion,
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    getToolVersions: mocks.getToolVersions,
    probeToolInstallations: mocks.probeToolInstallations,
    runToolLifecycleAction: mocks.runToolLifecycleAction,
  },
}));

vi.mock("@/lib/platform", () => ({
  isWindows: () => false,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: mocks.toastSuccess,
    error: mocks.toastError,
    info: mocks.toastInfo,
    warning: mocks.toastWarning,
  },
}));

vi.mock("@/components/settings/ToolUpgradeConfirmDialog", () => ({
  ToolUpgradeConfirmDialog: () => null,
}));

const toolVersion = (name: string) => ({
  name,
  version: name === "claude" || name === "codex" ? "1.0.0" : "2.0.0",
  latest_version: name === "claude" || name === "codex" ? "2.0.0" : "2.0.0",
  error: null,
  installed_but_broken: false,
  env_type: "linux" as const,
  wsl_distro: null,
});

describe("AboutSection", () => {
  beforeEach(() => {
    mocks.getVersion.mockResolvedValue("0.1.0");
    mocks.getToolVersions.mockImplementation(async (tools: string[]) =>
      tools.map(toolVersion),
    );
    mocks.probeToolInstallations.mockResolvedValue([]);
    mocks.runToolLifecycleAction.mockResolvedValue(undefined);
  });

  it("keeps Codex CLI read-only while preserving versions and other tool actions", async () => {
    const user = userEvent.setup();

    render(<AboutSection isPortable={false} />);

    expect(
      screen.queryByRole("button", { name: "settings.checkForUpdates" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "settings.releaseNotes" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "settings.officialWebsite" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "settings.github" }),
    ).not.toBeInTheDocument();

    await waitFor(() => {
      expect(
        mocks.getToolVersions.mock.calls.some(
          ([tools]) => Array.isArray(tools) && tools.includes("codex"),
        ),
      ).toBe(true);
    });

    const codexCard = screen
      .getByText("Codex CLI")
      .closest("div[class*='min-h']");
    expect(codexCard).not.toBeNull();
    await waitFor(() => {
      expect(
        within(codexCard as HTMLElement).getByText("1.0.0"),
      ).toBeInTheDocument();
    });
    expect(
      within(codexCard as HTMLElement).queryByRole("button", {
        name: "settings.toolInstall",
      }),
    ).not.toBeInTheDocument();
    expect(
      within(codexCard as HTMLElement).queryByRole("button", {
        name: "settings.toolUpdate",
      }),
    ).not.toBeInTheDocument();

    const claudeCard = screen
      .getByText("Claude Code")
      .closest("div[class*='min-h']");
    expect(claudeCard).not.toBeNull();
    await waitFor(() => {
      expect(
        within(claudeCard as HTMLElement).getByRole("button", {
          name: "settings.toolUpdate",
        }),
      ).toBeEnabled();
    });

    await user.click(
      screen.getByRole("button", { name: "settings.manualInstallCommands" }),
    );
    const commands = screen.getByText(/# Claude Code/).closest("pre");
    expect(commands?.textContent).not.toContain("@openai/codex");
    expect(commands?.textContent).not.toContain("# Codex");

    const updateAll = screen.getByRole("button", {
      name: "settings.updateAllTools",
    });
    await waitFor(() => expect(updateAll).toBeEnabled());
    await user.click(updateAll);

    await waitFor(() => {
      expect(mocks.probeToolInstallations).toHaveBeenCalledWith(["claude"]);
      expect(mocks.runToolLifecycleAction).toHaveBeenCalledWith(
        ["claude"],
        "update",
        {},
      );
    });
  });
});
