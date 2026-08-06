import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getWindowLayoutMode,
  type WindowLayoutMode,
  WINDOW_LAYOUT_POLICY,
} from "./windowLayoutConstants";

const readRendererWidth = () =>
  typeof window === "undefined" ? 0 : window.innerWidth;

const isWindowLayoutMode = (value: unknown): value is WindowLayoutMode =>
  value === "normal" || value === "constrained";

/**
 * Uses renderer width as a safe fallback until the native window host provides
 * an authoritative work-area event. The policy remains deliberately pure so
 * the host can apply the same normal/constrained decision before first paint.
 */
export function useWindowLayoutMode(): WindowLayoutMode {
  const [layoutMode, setLayoutMode] = useState<WindowLayoutMode>(() =>
    getWindowLayoutMode(readRendererWidth()),
  );

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    let active = true;
    let unlistenNativeLayoutMode: (() => void) | undefined;
    const syncLayoutMode = () => {
      setLayoutMode(getWindowLayoutMode(readRendererWidth()));
    };
    const scheduleLayoutModeSync = () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      timeoutId = setTimeout(
        syncLayoutMode,
        WINDOW_LAYOUT_POLICY.resizeDebounceMs,
      );
    };

    const subscribeToNativeLayoutMode = async () => {
      try {
        unlistenNativeLayoutMode = await listen<unknown>(
          "layout-mode-changed",
          ({ payload }) => {
            if (active && isWindowLayoutMode(payload)) {
              setLayoutMode(payload);
            }
          },
        );
      } catch {
        // Renderer-width fallback remains authoritative when no desktop host is
        // available (browser previews and fake tests included).
      }
    };

    void subscribeToNativeLayoutMode();
    window.addEventListener("resize", scheduleLayoutModeSync);
    return () => {
      active = false;
      window.removeEventListener("resize", scheduleLayoutModeSync);
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      unlistenNativeLayoutMode?.();
    };
  }, []);

  return layoutMode;
}
