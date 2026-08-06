import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  clampWindowGeometry,
  getDefaultWindowSize,
  getEffectiveMinimumSize,
  getWindowLayoutMode,
  WINDOW_LAYOUT_POLICY,
} from "@/lib/layout/windowLayoutConstants";

describe("window layout policy", () => {
  it("keeps the normal capacity policy versioned and aligned with Windows defaults", () => {
    const configPath = path.join(
      process.cwd(),
      "src-tauri",
      "tauri.windows.conf.json",
    );
    const config = JSON.parse(readFileSync(configPath, "utf8")) as {
      app: { windows: Array<Record<string, number>> };
    };
    const mainWindow = config.app.windows[0];

    expect(WINDOW_LAYOUT_POLICY.layoutVersion).toBe(2);
    expect(WINDOW_LAYOUT_POLICY.targetMinWidth).toBe(1152);
    expect(WINDOW_LAYOUT_POLICY.defaultWidth).toBe(1232);
    expect(WINDOW_LAYOUT_POLICY.defaultHeight).toBe(700);
    expect(mainWindow.width).toBe(WINDOW_LAYOUT_POLICY.defaultWidth);
    expect(mainWindow.height).toBe(WINDOW_LAYOUT_POLICY.defaultHeight);
    expect(mainWindow.minWidth).toBe(
      WINDOW_LAYOUT_POLICY.constrainedBaselineWidth,
    );
    expect(mainWindow.minHeight).toBe(
      WINDOW_LAYOUT_POLICY.constrainedBaselineHeight,
    );
  });

  it("uses normal mode when the workspace fits the target minimum", () => {
    const workArea = { x: 0, y: 0, width: 1600, height: 1000 };

    expect(getWindowLayoutMode(workArea.width)).toBe("normal");
    expect(getEffectiveMinimumSize(workArea)).toEqual({
      width: WINDOW_LAYOUT_POLICY.targetMinWidth,
      height: WINDOW_LAYOUT_POLICY.targetMinHeight,
    });
    expect(getDefaultWindowSize(workArea)).toEqual({
      width: WINDOW_LAYOUT_POLICY.defaultWidth,
      height: WINDOW_LAYOUT_POLICY.defaultHeight,
    });
  });

  it("reduces the effective minimum in a constrained workspace without exceeding it", () => {
    const workArea = { x: 0, y: 0, width: 1000, height: 650 };

    expect(getWindowLayoutMode(workArea.width)).toBe("constrained");
    expect(getEffectiveMinimumSize(workArea)).toEqual({
      width: 900,
      height: 585,
    });
    expect(getDefaultWindowSize(workArea)).toEqual({
      width: 900,
      height: 585,
    });
  });

  it("clamps off-screen or corrupt saved geometry and preserves maximization", () => {
    expect(
      clampWindowGeometry(
        { x: -900, y: 900, width: 6000, height: -1, maximized: true },
        { x: 100, y: 50, width: 1600, height: 1000 },
      ),
    ).toEqual({
      x: 100,
      y: 350,
      width: 1440,
      height: 700,
      maximized: true,
    });
  });
});
