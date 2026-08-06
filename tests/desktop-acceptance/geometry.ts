export interface DesktopRect {
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  opacity: number;
  pointerEvents: "auto" | "none";
}

export interface DesktopGeometryProbe {
  viewport: {
    width: number;
    height: number;
    scrollWidth: number;
  };
  elements: Record<string, DesktopRect>;
  tabStops: readonly string[];
}

export interface GeometryViolation {
  code:
    | "missing-element"
    | "outside-viewport"
    | "not-interactive"
    | "overlap"
    | "horizontal-overflow";
  target: string;
}

function right(rect: DesktopRect): number {
  return rect.x + rect.width;
}

function bottom(rect: DesktopRect): number {
  return rect.y + rect.height;
}

function overlaps(first: DesktopRect, second: DesktopRect): boolean {
  return (
    first.x < right(second) &&
    right(first) > second.x &&
    first.y < bottom(second) &&
    bottom(first) > second.y
  );
}

export function collectGeometryViolations(
  probe: DesktopGeometryProbe,
  requiredInteractiveElements: readonly string[],
  nonOverlappingPairs: readonly (readonly [string, string])[],
): GeometryViolation[] {
  const violations: GeometryViolation[] = [];

  if (probe.viewport.scrollWidth > probe.viewport.width) {
    violations.push({ code: "horizontal-overflow", target: "viewport" });
  }

  for (const name of requiredInteractiveElements) {
    const element = probe.elements[name];
    if (!element) {
      violations.push({ code: "missing-element", target: name });
      continue;
    }

    if (
      element.x < 0 ||
      element.y < 0 ||
      right(element) > probe.viewport.width ||
      bottom(element) > probe.viewport.height
    ) {
      violations.push({ code: "outside-viewport", target: name });
    }

    if (
      !element.visible ||
      element.opacity !== 1 ||
      element.pointerEvents !== "auto"
    ) {
      violations.push({ code: "not-interactive", target: name });
    }
  }

  for (const [firstName, secondName] of nonOverlappingPairs) {
    const first = probe.elements[firstName];
    const second = probe.elements[secondName];
    if (first && second && overlaps(first, second)) {
      violations.push({
        code: "overlap",
        target: `${firstName}:${secondName}`,
      });
    }
  }

  return violations;
}

export function isWithinOneCssPixel(before: number, after: number): boolean {
  return Math.abs(after - before) <= 1;
}

export function findForbiddenTabStops(
  tabStops: readonly string[],
  forbidden: readonly string[],
): string[] {
  const forbiddenSet = new Set(forbidden);
  return tabStops.filter((tabStop) => forbiddenSet.has(tabStop));
}
