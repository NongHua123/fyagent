import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { WorkBuddyDuplicateConflictDialog } from "@/components/workbuddy/WorkBuddyDuplicateConflictDialog";

describe("WorkBuddyDuplicateConflictDialog", () => {
  it("focuses cancellation by default and lets Escape cancel without updating duplicates", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const onConfirm = vi.fn();

    render(
      <WorkBuddyDuplicateConflictDialog
        duplicates={[{ id: "duplicate-model", count: 2 }]}
        isOpen
        isSaving={false}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    );

    const [cancelButton] = screen.getAllByRole("button");
    await waitFor(() => expect(cancelButton).toHaveFocus());

    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });
});
