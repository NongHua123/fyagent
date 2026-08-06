import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppSwitcher, getAppSwitcherCapacity } from "@/components/AppSwitcher";
import type { VisibleApps } from "@/types";

const legacyVisibleApps: VisibleApps = {
  claude: true,
  "claude-desktop": true,
  codex: true,
  // FyAgent versions before 0.1 have no `workbuddy` key.
  gemini: true,
  grokbuild: true,
  opencode: true,
  openclaw: true,
  hermes: true,
};

describe("AppSwitcher", () => {
  it("shows WorkBuddy by default after Codex for legacy visibility settings", () => {
    render(
      <AppSwitcher
        activeApp="claude"
        onSwitch={vi.fn()}
        visibleApps={legacyVisibleApps}
      />,
    );

    const labels = screen
      .getAllByRole("button")
      .map((button) => button.getAttribute("aria-label"));
    const codexIndex = labels.findIndex((label) => label?.includes("Codex"));
    const workBuddyIndex = labels.indexOf("apps.workbuddy");
    const geminiIndex = labels.findIndex((label) => label?.includes("Gemini"));

    expect(workBuddyIndex).toBe(codexIndex + 1);
    expect(workBuddyIndex).toBe(geminiIndex - 1);
  });

  it("keeps the active app direct and moves non-current apps into More in constrained mode", () => {
    const onSwitch = vi.fn();
    render(
      <AppSwitcher
        activeApp="hermes"
        layoutMode="constrained"
        onSwitch={onSwitch}
        visibleApps={legacyVisibleApps}
      />,
    );

    expect(screen.getByRole("button", { name: "Hermes" })).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "Claude Code" }),
    ).not.toBeInTheDocument();

    const more = screen.getByTestId("app-switcher-more");
    more.focus();
    fireEvent.keyDown(more, { key: "Enter" });

    const codex = screen.getByRole("menuitem", { name: "Codex" });
    expect(codex).toBeVisible();
    fireEvent.click(codex);
    expect(onSwitch).toHaveBeenCalledWith("codex");
  });

  it("reserves the overflow control before calculating direct apps", () => {
    const apps = ["claude", "codex", "workbuddy", "hermes"] as const;

    expect(getAppSwitcherCapacity(apps, "hermes", 0)).toEqual({
      directApps: ["hermes"],
      overflowApps: ["claude", "codex", "workbuddy"],
    });
    expect(getAppSwitcherCapacity(apps, "hermes", 200)).toEqual({
      directApps: ["claude", "codex", "hermes"],
      overflowApps: ["workbuddy"],
    });
    expect(getAppSwitcherCapacity(apps, "hermes", 212)).toEqual({
      directApps: ["claude", "codex", "workbuddy", "hermes"],
      overflowApps: [],
    });
  });
});
