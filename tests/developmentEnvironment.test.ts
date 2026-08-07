import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { parse as parseToml } from "smol-toml";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const SUPPORTED_PLATFORMS = [
  "linux-x64",
  "linux-arm64",
  "macos-x64",
  "macos-arm64",
  "windows-x64",
  "windows-arm64",
] as const;

type Artifact = { checksum?: string; url?: string };
type ToolEntry = {
  backend?: string;
  version?: string;
  options?: Record<string, string>;
  [key: string]: unknown;
};

function read(relativePath: string): string {
  return fs
    .readFileSync(path.join(ROOT, relativePath), "utf8")
    .replace(/\r\n/g, "\n");
}

function toml(relativePath: string): Record<string, unknown> {
  return parseToml(read(relativePath)) as Record<string, unknown>;
}

function lockEntry(lock: Record<string, unknown>, name: string): ToolEntry {
  const tools = lock.tools as Record<string, ToolEntry[]>;
  expect(tools[name]).toHaveLength(1);
  return tools[name][0];
}

describe("mise and uv development environment", () => {
  it("keeps each standard tool version in one authoritative project source", () => {
    const packageJson = JSON.parse(read("package.json")) as {
      packageManager: string;
    };
    const rust = toml("rust-toolchain.toml") as {
      toolchain: {
        channel: string;
        components: string[];
        profile: string;
      };
    };
    const cargo = toml("src-tauri/Cargo.toml") as {
      package: { "rust-version": string };
    };

    expect(read(".node-version").trim()).toBe("24.19.0");
    expect(packageJson.packageManager).toBe("pnpm@10.12.3");
    expect(rust.toolchain).toEqual({
      channel: "1.97.1",
      components: ["rustfmt", "clippy"],
      profile: "minimal",
    });
    expect(read(".python-version").trim()).toBe("3.14.7");
    expect(cargo.package["rust-version"]).toBe("1.85.0");
  });

  it("lets mise manage only uv directly and loads idiomatic version sources", () => {
    const config = toml("mise.toml") as {
      settings: {
        idiomatic_version_file_enable_tools: string[];
        lockfile_platforms: string[];
        task: { run_auto_install: boolean };
      };
      tools: Record<string, string>;
      tool_alias: Record<string, string>;
    };

    expect(config.tools).toEqual({ uv: "latest" });
    expect(config.tool_alias).toEqual({
      pnpm: "github:pnpm/pnpm",
      uv: "github:astral-sh/uv",
    });
    expect(config.settings.idiomatic_version_file_enable_tools).toEqual([
      "node",
      "pnpm",
      "rust",
    ]);
    expect(config.settings.lockfile_platforms).toEqual(SUPPORTED_PLATFORMS);
    expect(config.settings.task.run_auto_install).toBe(false);

    const loaded = JSON.parse(
      execFileSync("mise", ["config", "ls", "--json"], {
        cwd: ROOT,
        encoding: "utf8",
      }),
    ) as Array<{ path: string; tools: string[] }>;
    const byPath = new Map(
      loaded.map((entry) => [path.resolve(entry.path), entry.tools]),
    );
    expect(byPath.get(path.join(ROOT, "mise.toml"))).toEqual(["uv"]);
    expect(byPath.get(path.join(ROOT, ".node-version"))).toEqual(["node"]);
    expect(byPath.get(path.join(ROOT, "package.json"))).toEqual(["pnpm"]);
    expect(byPath.get(path.join(ROOT, "rust-toolchain.toml"))).toEqual([
      "rust",
    ]);
  });

  it("locks native Node, pnpm, and uv artifacts for all six platforms", () => {
    const lock = toml("mise.lock");
    const node = lockEntry(lock, "node");
    const pnpm = lockEntry(lock, "pnpm");
    const uv = lockEntry(lock, "uv");

    expect(node.backend).toBe("core:node");
    expect(node.version).toBe("24.19.0");
    expect(pnpm.backend).toBe("github:pnpm/pnpm");
    expect(pnpm.version).toBe("10.12.3");
    expect(uv.backend).toBe("github:astral-sh/uv");
    expect(uv.version).toMatch(/^\d+\.\d+\.\d+$/);

    for (const [name, entry] of Object.entries({ node, pnpm, uv })) {
      for (const platform of SUPPORTED_PLATFORMS) {
        const artifact = entry[`platforms.${platform}`] as Artifact;
        expect(artifact, `${name} ${platform}`).toBeDefined();
        expect(artifact.checksum, `${name} ${platform}`).toMatch(
          /^sha256:[a-f0-9]{64}$/,
        );
        expect(artifact.url, `${name} ${platform}`).toMatch(/^https:\/\//);
        const architecture = platform.endsWith("arm64")
          ? /(?:arm64|aarch64)/i
          : /(?:x64|x86_64)/i;
        expect(artifact.url, `${name} ${platform}`).toMatch(architecture);
      }
    }

    expect((pnpm["platforms.windows-arm64"] as Artifact).url).toContain(
      "pnpm-win-arm64.exe",
    );
    expect((uv["platforms.windows-arm64"] as Artifact).url).toContain(
      "uv-aarch64-pc-windows-msvc.zip",
    );
  });

  it("records the Rust core-backend limitation without inventing platform assets", () => {
    const lock = toml("mise.lock");
    const rust = lockEntry(lock, "rust");

    expect(rust).toMatchObject({
      backend: "core:rust",
      version: "1.97.1",
      options: { components: "clippy,rustfmt", profile: "minimal" },
    });
    expect(
      Object.keys(rust).filter((key) => key.startsWith("platforms.")),
    ).toEqual([]);
    expect(read("mise.lock")).not.toMatch(/llvm-tools|^targets\s*=/m);
  });

  it("uses a non-package uv project and ignores only local environment overlays", () => {
    const project = toml("pyproject.toml") as {
      project: {
        name: string;
        version: string;
        "requires-python": string;
        dependencies: unknown[];
      };
      "dependency-groups": { dev: unknown[] };
      tool: {
        uv: {
          package: boolean;
          "python-preference": string;
          "python-downloads": string;
        };
      };
    };

    expect(project.project).toEqual({
      name: "fyagent-development-environment",
      version: "0.0.0",
      "requires-python": ">=3.14,<3.15",
      dependencies: [],
    });
    expect(project["dependency-groups"].dev).toEqual([]);
    expect(project.tool.uv).toEqual({
      package: false,
      "python-preference": "only-managed",
      "python-downloads": "automatic",
    });
    expect(read("uv.lock")).toContain('requires-python = "==3.14.*"');

    const ignored = read(".gitignore");
    for (const entry of [
      ".venv/",
      "mise.local.toml",
      "mise.local.lock",
      "mise.*.local.toml",
      "mise.*.local.lock",
    ]) {
      expect(ignored).toContain(entry);
    }
  });

  it("passes the executable lockfile architecture and source contract", () => {
    const result = execFileSync(
      process.execPath,
      ["scripts/tasks/lockfile-check.mjs"],
      { cwd: ROOT, encoding: "utf8" },
    );
    const report = JSON.parse(result) as {
      ok: boolean;
      artifactPlatforms: string[];
      rustLockCoverage: string;
    };

    expect(report.ok).toBe(true);
    expect(report.artifactPlatforms).toEqual(SUPPORTED_PLATFORMS);
    expect(report.rustLockCoverage).toContain("exact-version-and-options");
  });
});
