import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const read = (relative: string) =>
  fs.readFileSync(path.join(ROOT, relative), "utf8").replace(/\r\n/g, "\n");

const RETIRED_TASKS = [
  "macos:preflight",
  "build:cross-windows:x64",
  "build:cross-windows:arm64",
  "build:cross-windows",
  "build:cross-macos:universal",
];
const RETIRED_PATHS = ["scripts/macos-cross", "scripts/windows-cross"];
const CURRENT_DOCUMENTS = [
  "README.md",
  "README_ZH.md",
  "README_JA.md",
  "README_DE.md",
  "CONTRIBUTING.md",
  ".trellis/spec/backend/index.md",
  ".trellis/spec/backend/development-environment.md",
  ".trellis/spec/backend/fyagent-version-contract.md",
  ".trellis/spec/backend/windows-release-boundary.md",
];

function executableRepositoryFiles(): string[] {
  const roots = [
    "mise.toml",
    "mise.lock",
    "scripts",
    ".github/workflows",
    ".trellis/scripts",
    ".mise",
    ".codex",
  ];
  const files: string[] = [];
  for (const relativeRoot of roots) {
    const absoluteRoot = path.join(ROOT, relativeRoot);
    if (!fs.existsSync(absoluteRoot)) continue;
    if (fs.statSync(absoluteRoot).isFile()) {
      files.push(absoluteRoot);
      continue;
    }
    const visit = (directory: string) => {
      for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        const absolute = path.join(directory, entry.name);
        if (entry.isDirectory()) visit(absolute);
        else if (entry.isFile()) files.push(absolute);
      }
    };
    visit(absoluteRoot);
  }
  return files;
}

describe("local build boundary", () => {
  it("removes every local cross-OS build entrypoint and dedicated contract", () => {
    for (const retiredPath of RETIRED_PATHS) {
      expect(fs.existsSync(path.join(ROOT, retiredPath))).toBe(false);
    }
    expect(
      fs.existsSync(path.join(ROOT, "tests/macosCrossWorkflow.test.ts")),
    ).toBe(false);
    expect(
      fs.existsSync(
        path.join(ROOT, ".trellis/spec/backend/wsl-macos-cross-build.md"),
      ),
    ).toBe(false);

    const taskSources = [
      read("mise.toml"),
      ...fs
        .readdirSync(path.join(ROOT, ".mise/tasks"))
        .filter((name) => name.endsWith(".toml"))
        .map((name) => read(path.posix.join(".mise/tasks", name))),
    ].join("\n");
    for (const task of RETIRED_TASKS) {
      expect(taskSources).not.toContain(`[tasks."${task}"]`);
      expect(taskSources).not.toContain(`["${task}"]`);
    }
    for (const retiredPath of RETIRED_PATHS) {
      expect(taskSources).not.toContain(retiredPath);
    }
    expect(taskSources).not.toContain("llvm-tools");
    expect(taskSources).not.toMatch(/^targets\s*=/m);

    const lock = read("mise.lock");
    expect(lock).not.toContain("llvm-tools");
    expect(lock).not.toMatch(/^targets\s*=/m);
  });

  it("keeps standard local development and builds current-host-only", () => {
    const packageJson = JSON.parse(read("package.json")) as {
      scripts: Record<string, string>;
    };
    expect(packageJson.scripts.dev).toBe("pnpm tauri dev");
    expect(packageJson.scripts.build).toBe("pnpm tauri build");
    expect(packageJson.scripts.dev).not.toContain("--target");
    expect(packageJson.scripts.build).not.toContain("--target");

    const nativeTasks = read(".mise/tasks/core.toml");
    for (const task of ["dev", "build", "build:binary", "build:debug"]) {
      expect(nativeTasks).toContain(
        task.includes(":") ? `["${task}"]` : `[${task}]`,
      );
    }
    expect(nativeTasks).not.toContain("--target");
    expect(read(".mise/tasks/rust.toml")).toContain(
      'run = "node scripts/tasks/rust.mjs test"',
    );

    for (const document of CURRENT_DOCUMENTS.slice(0, 4)) {
      const content = read(document);
      expect(content).toContain("mise exec -- pnpm dev");
      expect(content).toContain("mise exec -- pnpm build");
      expect(content).not.toContain("dist-bundle/");
    }
  });

  it("prevents repository tasks, scripts, and hooks from changing mise trust", () => {
    const miseTrustMutation =
      /(?:\bmise(?:\.exe)?\b|\/[^\s"'`]*\/mise\b|[A-Za-z]:\\[^\s"'`]*\\mise(?:\.exe)?\b|\$\{?[A-Za-z_][A-Za-z0-9_]*MISE[A-Za-z0-9_]*\}?)[^\r\n]*\b(?:trust|untrust)\b/i;
    for (const file of executableRepositoryFiles()) {
      const relative = path.relative(ROOT, file);
      const content = read(relative);
      expect(content, relative).not.toMatch(miseTrustMutation);
    }
  });

  it("keeps current documents free of retired cross-build interfaces", () => {
    for (const document of CURRENT_DOCUMENTS) {
      const content = read(document);
      for (const task of RETIRED_TASKS) {
        expect(content, document).not.toContain(task);
      }
      for (const retiredPath of RETIRED_PATHS) {
        expect(content, document).not.toContain(retiredPath);
      }
      expect(content, document).not.toContain("wsl-macos-cross-build.md");
    }
  });

  it("retains native release targets for all five platform groups", () => {
    const release = read(".github/workflows/release.yml");
    for (const runner of [
      '"os":"windows-2022"',
      '"os":"windows-11-arm","arch":"arm64"',
      '"os":"ubuntu-22.04"',
      '"os":"ubuntu-22.04-arm","arch":"arm64"',
      '"os":"macos-14"',
    ]) {
      expect(release).toContain(runner);
    }
    expect(release).toContain(
      "pnpm tauri build --target aarch64-pc-windows-msvc --no-bundle",
    );
    expect(release).toContain("pnpm tauri build --no-bundle");
    expect(release).toContain("pnpm tauri build --bundles appimage,deb,rpm");
    expect(release).toContain(
      "pnpm tauri build --target universal-apple-darwin",
    );
    expect(release).toContain("FYAGENT_WINDOWS_MANIFEST: release");
  });
});
