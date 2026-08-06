import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const WORKFLOWS_DIR = path.resolve(__dirname, "..", ".github", "workflows");

function readWorkflow(name: string): string {
  return fs
    .readFileSync(path.join(WORKFLOWS_DIR, name), "utf8")
    .replace(/\r\n/g, "\n");
}

function readHeaderBefore(source: string, marker: string): string {
  const markerIndex = source.indexOf(marker);
  expect(markerIndex).toBeGreaterThan(-1);
  return source.slice(0, markerIndex).trimEnd();
}

describe("GitHub workflow trigger policy", () => {
  it("runs frontend and backend CI only by manual dispatch", () => {
    const source = readWorkflow("ci.yml");
    const triggerSection = readHeaderBefore(source, "\nconcurrency:");

    expect(triggerSection).toBe(
      ["name: CI", "", "on:", "  workflow_dispatch:"].join("\n"),
    );
  });

  it("keeps desktop acceptance in the manual CI path and mock-only boundary", () => {
    const source = readWorkflow("ci.yml");

    expect(source).toContain("desktop-acceptance-contract:");
    expect(source).toContain("run: pnpm test:desktop:mock");
    expect(source).toContain("run: pnpm test:desktop:visual:preflight");
    expect(source).not.toContain("run: pnpm test:e2e");
  });

  it("labels a selected pull request only by manual dispatch", () => {
    const source = readWorkflow("labeler.yml");
    const triggerSection = readHeaderBefore(source, "\npermissions:");

    expect(triggerSection).toBe(
      [
        "name: Label PRs",
        "",
        "on:",
        "  workflow_dispatch:",
        "    inputs:",
        "      pr_number:",
        '        description: "Pull request number to label"',
        "        required: true",
        "        type: number",
      ].join("\n"),
    );
    expect(source).toContain("          pr-number: ${{ inputs.pr_number }}");
  });
});
