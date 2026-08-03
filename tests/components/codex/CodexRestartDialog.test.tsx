import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CodexRestartDialog } from "@/components/codex/CodexRestartDialog";

describe("CodexRestartDialog", () => {
  it("focuses the defer action and treats Escape as deferring a restart", async () => {
    const user = userEvent.setup();
    const onDefer = vi.fn();

    render(
      <CodexRestartDialog
        dialog={{ kind: "restart" }}
        isRestarting={false}
        onRestart={vi.fn()}
        onConfirmForceRestart={vi.fn()}
        onDefer={onDefer}
      />,
    );

    const [deferButton] = screen.getAllByRole("button");
    await waitFor(() => expect(deferButton).toHaveFocus());

    await user.keyboard("{Escape}");
    expect(onDefer).toHaveBeenCalledTimes(1);
  });

  it("does not close a restart request while its backend operation is pending", async () => {
    const user = userEvent.setup();
    const onDefer = vi.fn();

    render(
      <CodexRestartDialog
        dialog={{ kind: "restart" }}
        isRestarting
        onRestart={vi.fn()}
        onConfirmForceRestart={vi.fn()}
        onDefer={onDefer}
      />,
    );

    await user.keyboard("{Escape}");
    expect(onDefer).not.toHaveBeenCalled();
  });
});
