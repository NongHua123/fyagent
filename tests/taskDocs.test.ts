import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const DOCUMENT = path.join(
  ROOT,
  "docs",
  "fyagent",
  "development",
  "mise-tasks.md",
);
const GENERATOR = path.join(ROOT, "scripts", "tasks", "task-docs.mjs");

type Generator = {
  escapeMarkdownCell(value: unknown): string;
  generateTaskDocs(): string;
};

let generator: Generator;

beforeAll(async () => {
  generator = (await import(
    /* @vite-ignore */ pathToFileURL(GENERATOR).href
  )) as Generator;
});

describe("generated mise task documentation", () => {
  it("is a byte-for-byte rendering of live task metadata", () => {
    const document = fs.readFileSync(DOCUMENT, "utf8").replace(/\r\n/g, "\n");
    expect(document).toBe(generator.generateTaskDocs());
    expect(document).toContain(
      "> Generated from `.mise/tasks/*.toml` by `mise run tasks:docs:generate --apply`.",
    );

    const result = execFileSync(process.execPath, [GENERATOR, "check"], {
      cwd: ROOT,
      encoding: "utf8",
    });
    expect(result).toContain("byte-for-byte current");
  });

  it("documents every currently loaded task without freezing future extensions", () => {
    const tasks = JSON.parse(
      execFileSync("mise", ["tasks", "ls", "--local", "--json"], {
        cwd: ROOT,
        encoding: "utf8",
      }),
    ) as Array<{ name: string }>;
    const document = fs.readFileSync(DOCUMENT, "utf8");

    expect(tasks.length).toBeGreaterThanOrEqual(80);
    for (const task of tasks) {
      const escapedName = task.name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      expect(document, task.name).toMatch(
        new RegExp("\\| `" + escapedName + "` +\\|"),
      );
    }
  });

  it("escapes Markdown pipes and normalizes multiline metadata", () => {
    expect(generator.escapeMarkdownCell("left|right\n next")).toBe(
      "left\\|right next",
    );
  });
});
