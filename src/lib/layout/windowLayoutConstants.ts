export type WindowLayoutMode = "normal" | "constrained";

export interface LogicalSize {
  width: number;
  height: number;
}

export interface LogicalPosition {
  x: number;
  y: number;
}

export interface LogicalWorkArea extends LogicalPosition, LogicalSize {}

export interface WindowGeometry extends LogicalPosition, LogicalSize {
  maximized: boolean;
}

const WINDOW_LAYOUT_MEASUREMENT = {
  // This is the committed maximum-combination fixture result before the 32px
  // safety margin. It keeps normal-capacity sizing independent of transient DOM
  // measurements while the renderer is loading.
  requiredContentWidth: 1120,
  safetyMargin: 32,
  roundingIncrement: 8,
} as const;

const roundUpToIncrement = (value: number, increment: number) =>
  Math.ceil(value / increment) * increment;

const targetMinWidth = roundUpToIncrement(
  WINDOW_LAYOUT_MEASUREMENT.requiredContentWidth +
    WINDOW_LAYOUT_MEASUREMENT.safetyMargin,
  WINDOW_LAYOUT_MEASUREMENT.roundingIncrement,
);

export const WINDOW_LAYOUT_POLICY = {
  layoutVersion: 2,
  targetMinWidth,
  targetMinHeight: 640,
  defaultWidth: targetMinWidth + 80,
  defaultHeight: 700,
  constrainedBaselineWidth: 900,
  constrainedBaselineHeight: 600,
  maximumWorkAreaShare: 0.9,
  resizeDebounceMs: 150,
} as const;

const isFiniteNumber = (value: number) => Number.isFinite(value);

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(Math.max(value, minimum), maximum);

const maximumVisibleDimension = (dimension: number) =>
  Math.max(
    1,
    Math.floor(dimension * WINDOW_LAYOUT_POLICY.maximumWorkAreaShare),
  );

const fallbackWorkArea: LogicalWorkArea = {
  x: 0,
  y: 0,
  width: WINDOW_LAYOUT_POLICY.defaultWidth,
  height: WINDOW_LAYOUT_POLICY.defaultHeight,
};

const normalizedDimension = (value: number, fallback: number) =>
  isFiniteNumber(value) && value > 0 ? value : fallback;

const normalizedCoordinate = (value: number, fallback: number) =>
  isFiniteNumber(value) ? value : fallback;

export function normalizeLogicalWorkArea(
  workArea: Partial<LogicalWorkArea>,
): LogicalWorkArea {
  return {
    x: normalizedCoordinate(workArea.x ?? Number.NaN, fallbackWorkArea.x),
    y: normalizedCoordinate(workArea.y ?? Number.NaN, fallbackWorkArea.y),
    width: normalizedDimension(
      workArea.width ?? Number.NaN,
      fallbackWorkArea.width,
    ),
    height: normalizedDimension(
      workArea.height ?? Number.NaN,
      fallbackWorkArea.height,
    ),
  };
}

export function getWindowLayoutMode(workAreaWidth: number): WindowLayoutMode {
  return workAreaWidth >= WINDOW_LAYOUT_POLICY.targetMinWidth
    ? "normal"
    : "constrained";
}

export function getEffectiveMinimumSize(
  workAreaInput: Partial<LogicalWorkArea>,
): LogicalSize {
  const workArea = normalizeLogicalWorkArea(workAreaInput);
  return {
    width: Math.min(
      WINDOW_LAYOUT_POLICY.targetMinWidth,
      maximumVisibleDimension(workArea.width),
    ),
    height: Math.min(
      WINDOW_LAYOUT_POLICY.targetMinHeight,
      maximumVisibleDimension(workArea.height),
    ),
  };
}

export function getDefaultWindowSize(
  workAreaInput: Partial<LogicalWorkArea>,
): LogicalSize {
  const workArea = normalizeLogicalWorkArea(workAreaInput);
  const minimumSize = getEffectiveMinimumSize(workArea);
  return {
    width: clamp(
      WINDOW_LAYOUT_POLICY.defaultWidth,
      minimumSize.width,
      maximumVisibleDimension(workArea.width),
    ),
    height: clamp(
      WINDOW_LAYOUT_POLICY.defaultHeight,
      minimumSize.height,
      maximumVisibleDimension(workArea.height),
    ),
  };
}

/**
 * Gives the native host a deterministic migration target for saved window
 * state. The host applies the resulting geometry before it shows the window,
 * then restores maximization as its final state transition.
 */
export function clampWindowGeometry(
  geometryInput: Partial<WindowGeometry>,
  workAreaInput: Partial<LogicalWorkArea>,
): WindowGeometry {
  const workArea = normalizeLogicalWorkArea(workAreaInput);
  const minimumSize = getEffectiveMinimumSize(workArea);
  const defaultSize = getDefaultWindowSize(workArea);
  const maximumWidth = maximumVisibleDimension(workArea.width);
  const maximumHeight = maximumVisibleDimension(workArea.height);
  const width = clamp(
    normalizedDimension(geometryInput.width ?? Number.NaN, defaultSize.width),
    minimumSize.width,
    maximumWidth,
  );
  const height = clamp(
    normalizedDimension(geometryInput.height ?? Number.NaN, defaultSize.height),
    minimumSize.height,
    maximumHeight,
  );
  const maximumX = workArea.x + workArea.width - width;
  const maximumY = workArea.y + workArea.height - height;

  return {
    x: clamp(
      normalizedCoordinate(geometryInput.x ?? Number.NaN, workArea.x),
      workArea.x,
      maximumX,
    ),
    y: clamp(
      normalizedCoordinate(geometryInput.y ?? Number.NaN, workArea.y),
      workArea.y,
      maximumY,
    ),
    width,
    height,
    maximized: geometryInput.maximized === true,
  };
}
