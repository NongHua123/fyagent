import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { describe, expect, it } from "vitest";
import {
  DESKTOP_ACCEPTANCE_LOCALES,
  DESKTOP_ACCEPTANCE_PLATFORMS,
  DESKTOP_ACCEPTANCE_SCALES,
  MockDesktopBoundaryError,
  PROHIBITED_REAL_DESKTOP_OPERATIONS,
  createDesktopAcceptanceFixture,
  createMockDesktopInvoker,
} from "./fixtures";
import {
  collectGeometryViolations,
  findForbiddenTabStops,
  isWithinOneCssPixel,
} from "./geometry";
import requirementsMatrix from "./requirements-matrix.json";
import {
  createMockOnlyAcceptanceReport,
  redactAcceptanceDiagnostic,
} from "./reports";

const repositoryRoot = path.resolve(__dirname, "..", "..");
const visualManifestPath = path.join(
  repositoryRoot,
  "tests",
  "e2e",
  "visual-baselines",
  "manifest.json",
);
const gitAttributesPath = path.join(repositoryRoot, ".gitattributes");
const visualBaselineUpdateGatePath = path.join(
  repositoryRoot,
  "scripts",
  "desktop-acceptance",
  "review-visual-baseline-update.mjs",
);

describe("mock-only desktop acceptance contract", () => {
  it("uses a deterministic fixture for each documented locale, platform, and layout", () => {
    const normalFixture = createDesktopAcceptanceFixture();
    const constrainedFixture = createDesktopAcceptanceFixture({
      locale: "ja",
      platform: "macos",
      layout: "constrained",
    });

    expect(normalFixture).toEqual(createDesktopAcceptanceFixture());
    expect(normalFixture).toMatchObject({
      mode: "mock-only",
      now: "2026-01-01T00:00:00.000Z",
      timeZone: "UTC",
      randomSeed: 1024,
      network: "blocked",
      animations: "disabled",
      viewport: { width: 1440, height: 900 },
    });
    expect(constrainedFixture).toMatchObject({
      locale: "ja",
      platform: "macos",
      layout: "constrained",
      viewport: { width: 900, height: 600 },
    });
    expect(DESKTOP_ACCEPTANCE_LOCALES).toEqual(["en", "ja", "zh", "zh-TW"]);
    expect(DESKTOP_ACCEPTANCE_PLATFORMS).toEqual(["windows", "macos", "linux"]);
    expect(DESKTOP_ACCEPTANCE_SCALES).toEqual([100, 125, 150]);
  });

  it("uses an allowlist-only mock IPC boundary and never exposes real desktop operations", async () => {
    const fixture = createDesktopAcceptanceFixture();
    const invoke = createMockDesktopInvoker(fixture);

    await expect(invoke("desktop.acceptance.fixture")).resolves.toEqual(
      fixture,
    );
    await expect(invoke("desktop.acceptance.workbuddy")).resolves.toEqual(
      fixture.workbuddy,
    );
    await expect(invoke("restart-codex-desktop")).rejects.toBeInstanceOf(
      MockDesktopBoundaryError,
    );
    expect(PROHIBITED_REAL_DESKTOP_OPERATIONS).toEqual(
      expect.arrayContaining([
        "launch external desktop applications",
        "trigger UAC, installers, package managers, or registries",
      ]),
    );
  });

  it("provides reusable geometry, focus, and scroll guards for a future candidate runner", () => {
    const goodProbe = {
      viewport: { width: 1440, height: 900, scrollWidth: 1440 },
      elements: {
        appSwitcher: {
          x: 16,
          y: 12,
          width: 880,
          height: 40,
          visible: true,
          opacity: 1,
          pointerEvents: "auto" as const,
        },
        addProvider: {
          x: 1368,
          y: 12,
          width: 40,
          height: 40,
          visible: true,
          opacity: 1,
          pointerEvents: "auto" as const,
        },
      },
      tabStops: ["appSwitcher", "addProvider"],
    };

    expect(
      collectGeometryViolations(
        goodProbe,
        ["appSwitcher", "addProvider"],
        [["appSwitcher", "addProvider"]],
      ),
    ).toEqual([]);
    expect(isWithinOneCssPixel(600, 601)).toBe(true);
    expect(isWithinOneCssPixel(600, 602)).toBe(false);
    expect(
      findForbiddenTabStops(goodProbe.tabStops, ["workbuddySpacer"]),
    ).toEqual([]);

    const violations = collectGeometryViolations(
      {
        ...goodProbe,
        viewport: { ...goodProbe.viewport, scrollWidth: 1441 },
        elements: {
          ...goodProbe.elements,
          addProvider: {
            ...goodProbe.elements.addProvider,
            x: 1420,
            pointerEvents: "none",
          },
        },
      },
      ["addProvider"],
      [],
    );

    expect(violations.map((violation) => violation.code)).toEqual(
      expect.arrayContaining([
        "horizontal-overflow",
        "outside-viewport",
        "not-interactive",
      ]),
    );
  });

  it("maps every acceptance area to tracked evidence without treating candidate checks as passed", () => {
    const requirementIds = requirementsMatrix.entries.flatMap(
      (entry) => entry.requirements,
    );

    expect(requirementIds).toEqual(
      expect.arrayContaining([
        "WB-T01",
        "CR-T01",
        "CV-T01",
        "UI-E01",
        "WB-E01",
        "WIN-T01",
        "I18N-T01",
      ]),
    );
    for (const entry of requirementsMatrix.entries) {
      expect(entry.evidence.length).toBeGreaterThan(0);
      for (const evidence of entry.evidence) {
        expect(fs.existsSync(path.join(repositoryRoot, evidence))).toBe(true);
      }
    }

    const report = createMockOnlyAcceptanceReport();
    expect(report.automated).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: "QA-MOCK-001", status: "passed" }),
      ]),
    );
    expect(report.notRunInThisEnvironment).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: "desktop-candidate-e2e",
          status: "not-run",
        }),
        expect.objectContaining({
          id: "windows-release-candidate",
          status: "not-run",
        }),
      ]),
    );
  });

  it("keeps visual baselines isolated by platform, scale, and locale under Git LFS", () => {
    const manifest = JSON.parse(
      fs.readFileSync(visualManifestPath, "utf8"),
    ) as {
      captureMode: string;
      platforms: string[];
      scales: number[];
      locales: string[];
      stabilitySamples: number;
      regions: { id: string; pathTemplate: string }[];
    };

    expect(manifest.captureMode).toBe("candidate-only");
    expect(manifest.platforms).toEqual([...DESKTOP_ACCEPTANCE_PLATFORMS]);
    expect(manifest.scales).toEqual([...DESKTOP_ACCEPTANCE_SCALES]);
    expect(manifest.locales).toEqual([...DESKTOP_ACCEPTANCE_LOCALES]);
    expect(manifest.stabilitySamples).toBe(2);
    expect(manifest.regions.map((region) => region.id)).toEqual(
      expect.arrayContaining(["top-level-header", "workbuddy-page"]),
    );
    for (const region of manifest.regions) {
      expect(region.pathTemplate).toContain("{platform}/{scale}/{locale}/");
      expect(region.pathTemplate).toMatch(/\.png$/);
    }
    expect(fs.readFileSync(gitAttributesPath, "utf8")).toContain(
      "tests/e2e/visual-baselines/**/*.png filter=lfs diff=lfs merge=lfs -text",
    );
  });

  it("fails closed before a visual update can write a baseline", () => {
    const manifestBefore = fs.readFileSync(visualManifestPath, "utf8");
    const result = spawnSync(process.execPath, [visualBaselineUpdateGatePath], {
      cwd: repositoryRoot,
      encoding: "utf8",
      env: { PATH: process.env.PATH ?? "" },
    });

    expect(result.status).toBe(2);
    expect(result.stdout).toBe("");
    expect(result.stderr).toContain("Refusing visual baseline update");
    expect(fs.readFileSync(visualManifestPath, "utf8")).toBe(manifestBefore);
  });

  it("redacts credentials, process IDs, and private paths before diagnostics become evidence", () => {
    expect(
      redactAcceptanceDiagnostic(
        "apiKey=TEST-SECRET pid=4242 C:\\Users\\fixture-user\\settings.json",
      ),
    ).toBe("apiKey=[REDACTED] pid=[REDACTED] [REDACTED_PATH]");
  });
});
