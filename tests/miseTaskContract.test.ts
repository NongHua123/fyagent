import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");

type TaskDefinition = {
  confirm?: { default?: string; message?: string };
  env: { FYAGENT_TASK_EFFECT: string };
  interactive?: boolean;
  raw?: boolean;
  usage?: string;
};

type ContractModule = {
  PARAMETERIZED_TASKS: readonly string[];
  RAW_TASKS: readonly string[];
  loadTaskDefinitions(): Record<string, TaskDefinition>;
};

function mise(...args: string[]) {
  return spawnSync("mise", ["run", ...args], {
    cwd: ROOT,
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1" },
  });
}

function output(result: ReturnType<typeof mise>): string {
  return `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
}

function digest(relativePath: string): string {
  return createHash("sha256")
    .update(fs.readFileSync(path.join(ROOT, relativePath)))
    .digest("hex");
}

describe("canonical mise task API", () => {
  it("loads a complete and extensible catalog with valid metadata", () => {
    const validation = spawnSync(
      "mise",
      ["tasks", "validate", "--errors-only"],
      {
        cwd: ROOT,
        encoding: "utf8",
      },
    );
    expect(validation.status).toBe(0);
    expect(output(validation)).toContain("task(s) validated successfully");

    const contract = spawnSync(
      process.execPath,
      ["scripts/tasks/task-contract-check.mjs"],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(contract.status, output(contract)).toBe(0);
    const report = JSON.parse(contract.stdout) as {
      ok: boolean;
      tasks: number;
      checkClosure: string[];
    };
    expect(report.ok).toBe(true);
    expect(report.tasks).toBeGreaterThanOrEqual(80);
    expect(report.checkClosure).toContain("check:contracts");
  });

  it("enforces usage, mutation, interactive, raw, and confirmation metadata", async () => {
    const contract = (await import(
      /* @vite-ignore */ pathToFileURL(
        path.join(ROOT, "scripts", "tasks", "task-contract-check.mjs"),
      ).href
    )) as ContractModule;
    const tasks = contract.loadTaskDefinitions();

    for (const name of contract.PARAMETERIZED_TASKS) {
      expect(tasks[name].usage?.trim(), name).toBeTruthy();
    }
    for (const [name, task] of Object.entries(tasks)) {
      const effect = task.env.FYAGENT_TASK_EFFECT;
      if (effect === "preview-by-default") {
        expect(task.usage, name).toContain('flag "--apply"');
      }
      expect(task.interactive === true, name).toBe(effect === "interactive");
    }
    expect(
      Object.entries(tasks)
        .filter(([, task]) => task.raw === true)
        .map(([name]) => name)
        .sort(),
    ).toEqual([...contract.RAW_TASKS].sort());
    expect(tasks["upstream:merge:abort"]).toMatchObject({
      confirm: { default: "no" },
      env: { FYAGENT_TASK_EFFECT: "git-state" },
    });
  });

  it("forwards a unit-test file filter through the real mise usage parser", () => {
    const result = mise("test:unit", "tests/developmentEnvironment.test.ts");
    expect(result.status, output(result)).toBe(0);
    expect(output(result)).toContain("developmentEnvironment.test.ts");
    expect(output(result)).not.toContain("miseTaskContract.test.ts");
  }, 60_000);

  it("forwards version and Python parameters while preview mode preserves files", () => {
    const guardedFiles = [
      "src-tauri/Cargo.toml",
      "src-tauri/Cargo.lock",
      "pyproject.toml",
      "uv.lock",
    ];
    const before = new Map(
      guardedFiles.map((relativePath) => [relativePath, digest(relativePath)]),
    );

    const version = mise("version:set", "0.3.0");
    expect(version.status, output(version)).toBe(0);
    expect(output(version)).toContain("0.3.0");
    expect(output(version)).toMatch(/preview|no files (?:would )?change/i);

    const python = mise("python:add:dev", "httpx");
    expect(python.status, output(python)).toBe(0);
    expect(output(python)).toContain("httpx");
    expect(output(python)).toContain('"status": "preview"');

    for (const relativePath of guardedFiles) {
      expect(digest(relativePath), relativePath).toBe(before.get(relativePath));
    }
  });

  it("forwards upstream parameters before any Git mutation can run", () => {
    const result = mise("upstream:merge:prepare", "not-a-release-tag");
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("Upstream tag must be exact vX.Y.Z");
    expect(output(result)).not.toContain("git merge");
  });

  it("forwards flags to the JSON environment report", () => {
    const result = mise("env:check", "--json");
    expect(result.status, output(result)).toBe(0);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      checks: Array<{ name: string; ok: boolean }>;
    };
    expect(report.ok).toBe(true);
    expect(report.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "Node ownership", ok: true }),
        expect.objectContaining({
          name: "Rust toolchain and components",
          ok: true,
        }),
      ]),
    );
  });

  it.each([
    ["split target", ["--", "--target", "aarch64-unknown-linux-gnu"]],
    ["equals target", ["--", "--target=aarch64-unknown-linux-gnu"]],
  ])("rejects %s injection before Cargo runs", (_label, args) => {
    const result = mise("rust:test", ...args);
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("Cargo options and targets are forbidden");
    expect(output(result)).not.toMatch(/Compiling|Finished.*test profile/);
  });

  it.each(["--update", "--outputFile=vitest-results.json"])(
    "rejects the write-capable Vitest option %s before Vitest runs",
    (option) => {
      const result = mise("test:unit", "--", option);
      expect(result.status).not.toBe(0);
      expect(output(result)).toContain("Vitest options are forbidden");
      expect(output(result)).not.toContain("RUN ");
    },
  );
});
