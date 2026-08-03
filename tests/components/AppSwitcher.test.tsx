import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppSwitcher } from "@/components/AppSwitcher";
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
      .map((button) => button.textContent?.trim());
    const codexIndex = labels.findIndex((label) => label?.includes("Codex"));
    const workBuddyIndex = labels.indexOf("apps.workbuddy");
    const geminiIndex = labels.findIndex((label) => label?.includes("Gemini"));

    expect(workBuddyIndex).toBe(codexIndex + 1);
    expect(workBuddyIndex).toBe(geminiIndex - 1);
  });
});
