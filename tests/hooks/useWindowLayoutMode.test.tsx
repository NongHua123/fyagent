import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWindowLayoutMode } from "@/lib/layout/useWindowLayoutMode";
import { WINDOW_LAYOUT_POLICY } from "@/lib/layout/windowLayoutConstants";
import { emitTauriEvent } from "../msw/tauriMocks";

function LayoutModeProbe() {
  return <output>{useWindowLayoutMode()}</output>;
}

const originalInnerWidth = window.innerWidth;

afterEach(() => {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: originalInnerWidth,
    writable: true,
  });
  vi.useRealTimers();
});

describe("useWindowLayoutMode", () => {
  it("starts from the current renderer width and coalesces resize changes", () => {
    vi.useFakeTimers();
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: WINDOW_LAYOUT_POLICY.targetMinWidth,
      writable: true,
    });
    render(<LayoutModeProbe />);

    expect(screen.getByRole("status")).toHaveTextContent("normal");

    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 900,
      writable: true,
    });
    act(() => {
      window.dispatchEvent(new Event("resize"));
      window.dispatchEvent(new Event("resize"));
      vi.advanceTimersByTime(WINDOW_LAYOUT_POLICY.resizeDebounceMs - 1);
    });
    expect(screen.getByRole("status")).toHaveTextContent("normal");

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.getByRole("status")).toHaveTextContent("constrained");
  });

  it("accepts the native work-area mode when the host provides one", async () => {
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: WINDOW_LAYOUT_POLICY.targetMinWidth,
      writable: true,
    });
    render(<LayoutModeProbe />);

    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.getByRole("status")).toHaveTextContent("normal");

    act(() => {
      emitTauriEvent("layout-mode-changed", "constrained");
    });
    expect(screen.getByRole("status")).toHaveTextContent("constrained");

    act(() => {
      emitTauriEvent("layout-mode-changed", "unexpected-value");
    });
    expect(screen.getByRole("status")).toHaveTextContent("constrained");
  });
});
