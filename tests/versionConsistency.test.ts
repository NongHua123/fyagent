import { execFileSync } from "node:child_process";
import path from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(__dirname, "..");
const versionScript = path.join(repositoryRoot, "scripts", "version.mjs");

function runVersionCommand(
  command: "get" | "check",
  ...args: string[]
): string {
  return execFileSync(process.execPath, [versionScript, command, ...args], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
}

describe("FyAgent application version contract", () => {
  it("delegates repository metadata validation to the canonical version script", () => {
    const result = runVersionCommand("check").trim();
    expect(result).toMatch(/^FyAgent version contract OK: /);
  });

  it("prints the same canonical version consumed by the contract check", () => {
    const version = runVersionCommand("get").trim();
    const contract = runVersionCommand("check").trim();

    expect(contract).toBe("FyAgent version contract OK: " + version);
  });

  it("accepts only the tag derived from the canonical version", () => {
    const version = runVersionCommand("get").trim();
    const contract = runVersionCommand("check", "--tag", "v" + version).trim();

    expect(contract).toBe("FyAgent version contract OK: " + version);
  });
});
