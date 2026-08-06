export const DESKTOP_ACCEPTANCE_LOCALES = ["en", "ja", "zh", "zh-TW"] as const;
export const DESKTOP_ACCEPTANCE_PLATFORMS = [
  "windows",
  "macos",
  "linux",
] as const;
export const DESKTOP_ACCEPTANCE_SCALES = [100, 125, 150] as const;

export type DesktopAcceptanceLocale =
  (typeof DESKTOP_ACCEPTANCE_LOCALES)[number];
export type DesktopAcceptancePlatform =
  (typeof DESKTOP_ACCEPTANCE_PLATFORMS)[number];
export type DesktopAcceptanceLayout = "normal" | "constrained";

export interface DesktopAcceptanceFixture {
  mode: "mock-only";
  now: "2026-01-01T00:00:00.000Z";
  timeZone: "UTC";
  randomSeed: 1024;
  network: "blocked";
  fontFamily: "Noto Sans";
  animations: "disabled";
  locale: DesktopAcceptanceLocale;
  platform: DesktopAcceptancePlatform;
  layout: DesktopAcceptanceLayout;
  viewport: { width: number; height: number };
  versions: {
    local: "0.1.0";
    latest: "0.1.1";
  };
  workbuddy: {
    downloadUrl: "https://www.workbuddy.cn/";
    models: readonly { id: string; url: string; apiKey: "" }[];
  };
}

export type MockDesktopCommand =
  | "desktop.acceptance.fixture"
  | "desktop.acceptance.workbuddy";

export const PROHIBITED_REAL_DESKTOP_OPERATIONS = [
  "launch external desktop applications",
  "terminate or restart processes",
  "read user configuration or credentials",
  "trigger UAC, installers, package managers, or registries",
  "use a real network endpoint",
] as const;

export class MockDesktopBoundaryError extends Error {
  constructor() {
    super("The mock desktop boundary rejected an unavailable command.");
    this.name = "MockDesktopBoundaryError";
  }
}

function cloneFixture<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function createDesktopAcceptanceFixture(
  options: {
    locale?: DesktopAcceptanceLocale;
    platform?: DesktopAcceptancePlatform;
    layout?: DesktopAcceptanceLayout;
  } = {},
): DesktopAcceptanceFixture {
  const layout = options.layout ?? "normal";

  return {
    mode: "mock-only",
    now: "2026-01-01T00:00:00.000Z",
    timeZone: "UTC",
    randomSeed: 1024,
    network: "blocked",
    fontFamily: "Noto Sans",
    animations: "disabled",
    locale: options.locale ?? "zh",
    platform: options.platform ?? "windows",
    layout,
    viewport:
      layout === "normal"
        ? { width: 1440, height: 900 }
        : { width: 900, height: 600 },
    versions: {
      local: "0.1.0",
      latest: "0.1.1",
    },
    workbuddy: {
      downloadUrl: "https://www.workbuddy.cn/",
      models: [
        {
          id: "fixture-model-alpha",
          url: "https://fixture.invalid/v1",
          apiKey: "",
        },
      ],
    },
  };
}

export function createMockDesktopInvoker(
  fixture: DesktopAcceptanceFixture = createDesktopAcceptanceFixture(),
): (command: string) => Promise<unknown> {
  return async (command) => {
    switch (command as MockDesktopCommand) {
      case "desktop.acceptance.fixture":
        return cloneFixture(fixture);
      case "desktop.acceptance.workbuddy":
        return cloneFixture(fixture.workbuddy);
      default:
        throw new MockDesktopBoundaryError();
    }
  };
}
