import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CodexRestartDialog } from "@/components/codex/CodexRestartDialog";

describe("CodexRestartDialog", () => {
  it("uses the saved-config confirmation wording and focuses the manual action", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();

    render(
      <CodexRestartDialog
        dialog={{
          kind: "confirm",
          token: "opaque-token",
          reason: "multiple_instances",
        }}
        onConfirm={onConfirm}
        onRetry={vi.fn()}
        onDefer={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "需要重启 Codex" }),
    ).toBeVisible();
    expect(screen.getByText("配置已保存。")).toBeVisible();
    expect(screen.getByText("检测到多个正在运行的 Codex。")).toBeVisible();
    expect(
      screen.getByText(
        "继续后将强制关闭所有匹配的 Codex。未保存的工作可能丢失，随后只会启动一个实例。",
      ),
    ).toBeVisible();
    expect(
      screen.queryByText(/opaque-token|PID|phase/i),
    ).not.toBeInTheDocument();

    const manualButton = screen.getByRole("button", {
      name: "稍后手动重启",
    });
    await waitFor(() => expect(manualButton).toHaveFocus());

    await user.click(screen.getByRole("button", { name: "强制关闭并重启" }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("uses a neutral reason for a unique trusted runtime", () => {
    render(
      <CodexRestartDialog
        dialog={{
          kind: "confirm",
          token: "opaque-token",
          reason: "unique_runtime",
        }}
        onConfirm={vi.fn()}
        onRetry={vi.fn()}
        onDefer={vi.fn()}
      />,
    );

    expect(screen.getByText("Codex 正在运行。")).toBeVisible();
    expect(screen.queryByText(/多个/)).not.toBeInTheDocument();
  });

  it("keeps the progress view open for Escape or backdrop interaction and exposes no repeated action", async () => {
    const user = userEvent.setup();
    const onDefer = vi.fn();

    render(
      <CodexRestartDialog
        dialog={{ kind: "progress" }}
        onConfirm={vi.fn()}
        onRetry={vi.fn()}
        onDefer={onDefer}
      />,
    );

    expect(screen.getByText("正在重启 Codex…")).toBeVisible();
    expect(screen.queryAllByRole("button")).toHaveLength(0);

    await user.keyboard("{Escape}");
    fireEvent.pointerDown(document.body);
    fireEvent.pointerUp(document.body);

    expect(onDefer).not.toHaveBeenCalled();
    expect(screen.getByText("正在重启 Codex…")).toBeVisible();
  });

  it("presents the incomplete result with manual focus and a direct retry", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();

    render(
      <CodexRestartDialog
        dialog={{ kind: "incomplete", retryToken: "opaque-retry-token" }}
        onConfirm={vi.fn()}
        onRetry={onRetry}
        onDefer={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Codex 重启未完成" }),
    ).toBeVisible();
    expect(screen.getByText("配置已保存，但 Codex 重启未完成。")).toBeVisible();
    expect(
      screen.queryByText(/opaque-retry-token|PID|path|phase/i),
    ).not.toBeInTheDocument();

    const manualButton = screen.getByRole("button", {
      name: "我将手动重启",
    });
    await waitFor(() => expect(manualButton).toHaveFocus());

    await user.click(screen.getByRole("button", { name: "再次尝试重启" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });
});
