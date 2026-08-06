import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TopLevelHeader } from "@/components/topbar/TopLevelHeader";

const renderHeader = (trailingPrimaryActionEmpty = false) =>
  render(
    <TopLevelHeader
      actionsLabel="Actions"
      appSwitcher={<button type="button">App switcher</button>}
      contextActions={[
        {
          id: "skills",
          label: "Skills",
          icon: <span>icon</span>,
          onSelect: vi.fn(),
        },
      ]}
      leading={<span>FyAgent</span>}
      priorityControls={<button type="button">Profile</button>}
      trailingPrimaryAction={
        trailingPrimaryActionEmpty ? undefined : (
          <button type="button">Add provider</button>
        )
      }
      trailingPrimaryActionEmpty={trailingPrimaryActionEmpty}
    />,
  );

describe("TopLevelHeader", () => {
  it("keeps the trailing slot width when WorkBuddy replaces the action with an inert placeholder", () => {
    const { rerender } = renderHeader(false);
    const providerSlot = screen.getByTestId("trailing-primary-action-slot");

    expect(screen.getByRole("button", { name: "Add provider" })).toBeVisible();
    expect(providerSlot).toHaveClass("w-10", "shrink-0");

    rerender(
      <TopLevelHeader
        actionsLabel="Actions"
        appSwitcher={<button type="button">App switcher</button>}
        leading={<span>FyAgent</span>}
        trailingPrimaryActionEmpty
      />,
    );

    const workBuddySlot = screen.getByTestId("trailing-primary-action-slot");
    const placeholder = screen.getByTestId(
      "trailing-primary-action-placeholder",
    );
    expect(workBuddySlot.className).toBe(providerSlot.className);
    expect(placeholder).toHaveAttribute("aria-hidden", "true");
    expect(placeholder).not.toHaveAttribute("tabindex");
    expect(screen.queryByRole("button", { name: "Add provider" })).toBeNull();
  });

  it("exposes P2 actions through an accessible keyboard menu before the app capacity slot", () => {
    renderHeader();

    const actionTrigger = screen.getByRole("button", { name: "Actions" });
    expect(
      screen
        .getByTestId("top-level-actions-overflow")
        .compareDocumentPosition(
          screen.getByTestId("app-switcher-capacity-slot"),
        ) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    actionTrigger.focus();
    fireEvent.keyDown(actionTrigger, { key: "Enter" });
    expect(screen.getByRole("menuitem", { name: "Skills" })).toBeVisible();
  });
});
