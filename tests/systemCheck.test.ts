import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const SCRIPT = path.join(ROOT, "scripts", "tasks", "system-check.mjs");
const TOOLCHAIN_SCRIPT = path.join(
  ROOT,
  "scripts",
  "tasks",
  "toolchain-check.mjs",
);

function nodeScript(script: string, ...args: string[]) {
  return spawnSync(process.execPath, [script, ...args], {
    cwd: ROOT,
    encoding: "utf8",
  });
}

describe("read-only host prerequisite checks", () => {
  it.each(["linux", "darwin", "win32"])(
    "describes %s prerequisites without probing another host",
    (platform) => {
      const result = nodeScript(SCRIPT, "--describe-platform", platform);
      expect(result.status, result.stderr).toBe(0);
      const report = JSON.parse(result.stdout) as {
        platform: string;
        requirements: {
          commands: Array<[string, string[], string]>;
          pkgConfig: Array<[string, string]>;
        };
      };
      expect(report.platform).toBe(platform);
      expect(report.requirements.commands.length).toBeGreaterThan(0);
      for (const [command, args, hint] of report.requirements.commands) {
        expect(command).not.toMatch(/^(?:sudo|apt|brew|winget|choco)$/i);
        expect(args.join(" ")).not.toMatch(/(?:^|\s)(?:install|add)(?:\s|$)/i);
        expect(hint.length).toBeGreaterThan(0);
      }
    },
  );

  it("reports the current host as JSON and makes failures visible", () => {
    const result = spawnSync("mise", ["run", "system:check", "--json"], {
      cwd: ROOT,
      encoding: "utf8",
    });
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      platform: string;
      checks: Array<{ name: string; ok: boolean; hint?: string }>;
    };

    expect(report.platform).toBe(process.platform);
    expect(report.checks.length).toBeGreaterThan(0);
    expect(result.status === 0).toBe(report.ok);
    for (const check of report.checks.filter((entry) => !entry.ok)) {
      expect(check.hint).toBeTruthy();
    }
  });

  it("keeps reporting hints when every probed command is absent from PATH", () => {
    const result = spawnSync(process.execPath, [SCRIPT, "--json"], {
      cwd: ROOT,
      encoding: "utf8",
      env: { ...process.env, PATH: "" },
    });
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      checks: Array<{ name: string; ok: boolean; hint?: string }>;
    };

    expect(result.status).toBe(1);
    expect(report.ok).toBe(false);
    expect(report.checks.length).toBeGreaterThan(0);
    expect(
      report.checks.every((check) => !check.ok && Boolean(check.hint)),
    ).toBe(true);
  });

  it("contains no elevation or package-manager mutation command", () => {
    const source = fs.readFileSync(SCRIPT, "utf8");
    expect(source).not.toMatch(
      /run\(["'](?:sudo|apt|apt-get|brew|winget|choco)["']/i,
    );
    expect(source).not.toMatch(/exec(?:File)?Sync\(/);
  });

  it("normalizes native Windows separators before exact config-path comparison", () => {
    const backslashes = nodeScript(
      TOOLCHAIN_SCRIPT,
      "--normalize-path",
      "C:\\Repo\\FyAgent\\mise.toml",
    );
    const forwardSlashes = nodeScript(
      TOOLCHAIN_SCRIPT,
      "--normalize-path",
      "C:/Repo/FyAgent/mise.toml",
    );

    expect(backslashes.status, backslashes.stderr).toBe(0);
    expect(forwardSlashes.status, forwardSlashes.stderr).toBe(0);
    expect(backslashes.stdout.trim()).toBe(forwardSlashes.stdout.trim());
    const source = fs.readFileSync(TOOLCHAIN_SCRIPT, "utf8");
    expect(source).toContain("normalizeComparablePath(entry.path)");
    expect(source).not.toMatch(/entry\.path\.endsWith/);
    expect(source).toContain('capture("mise", ["which", "rustc"])');
    expect(source).toContain('capture("rustc", ["--print", "sysroot"])');
  });
});
