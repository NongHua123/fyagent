import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterAll, describe, expect, it } from "vitest";
// @ts-expect-error The workflow executes this dependency-free JavaScript helper directly.
import * as toolchainModule from "../scripts/ci/verify-toolchain.mjs";

type ToolchainContract = Readonly<{
  node: string;
  pnpm: string;
  rust: string;
  python: string;
  uv: string;
}>;

const ROOT = path.resolve(__dirname, "..");
const temporaryRoots: string[] = [];
const readToolchainContract = toolchainModule.readToolchainContract as (
  root?: string,
) => ToolchainContract;
const writeGithubOutputs = toolchainModule.writeGithubOutputs as (
  contract: ToolchainContract,
  outputPath: string,
) => void;

function temporaryContract(overrides: { miseLock?: string } = {}): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-ci-toolchain-"));
  temporaryRoots.push(root);
  fs.writeFileSync(path.join(root, ".node-version"), "25.1.2\n");
  fs.writeFileSync(
    path.join(root, "package.json"),
    JSON.stringify({ packageManager: "pnpm@11.2.3" }),
  );
  fs.writeFileSync(
    path.join(root, "rust-toolchain.toml"),
    '[toolchain]\nchannel = "2.3.4"\n',
  );
  fs.writeFileSync(path.join(root, ".python-version"), "3.15.1\n");
  fs.writeFileSync(
    path.join(root, "mise.lock"),
    overrides.miseLock ??
      '[[tools.uv]]\nversion = "1.2.3"\nbackend = "github:astral-sh/uv"\n',
  );
  return root;
}

afterAll(() => {
  for (const root of temporaryRoots) {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

describe("CI toolchain contract", () => {
  it("reads every expected version from its canonical repository source", () => {
    expect(readToolchainContract(ROOT)).toEqual({
      node: "24.19.0",
      pnpm: "10.12.3",
      rust: "1.97.1",
      python: "3.14.7",
      uv: "0.12.2",
    });
  });

  it("derives changed versions instead of carrying a second constant set", () => {
    const root = temporaryContract();
    expect(readToolchainContract(root)).toEqual({
      node: "25.1.2",
      pnpm: "11.2.3",
      rust: "2.3.4",
      python: "3.15.1",
      uv: "1.2.3",
    });

    const script = fs.readFileSync(
      path.join(ROOT, "scripts", "ci", "verify-toolchain.mjs"),
      "utf8",
    );
    for (const duplicate of [
      "24.19.0",
      "10.12.3",
      "1.97.1",
      "3.14.7",
      "0.12.2",
    ]) {
      expect(script).not.toContain(duplicate);
    }
  });

  it("fails closed when the uv lock selection is duplicated or malformed", () => {
    const root = temporaryContract({
      miseLock:
        '[[tools.uv]]\nversion = "1.2.3"\nbackend = "github:astral-sh/uv"\n' +
        '[[tools.uv]]\nversion = "1.2.4"\nbackend = "github:astral-sh/uv"\n',
    });
    expect(() => readToolchainContract(root)).toThrow(
      "mise.lock must contain exactly one [[tools.uv]] entry",
    );
  });

  it("emits locked uv and Python facts for setup-uv through GITHUB_OUTPUT", () => {
    const root = temporaryContract();
    const output = path.join(root, "github-output.txt");
    writeGithubOutputs(readToolchainContract(root), output);
    expect(fs.readFileSync(output, "utf8")).toBe(
      [
        "node-version=25.1.2",
        "pnpm-version=11.2.3",
        "rust-version=2.3.4",
        "python-version=3.15.1",
        "uv-version=1.2.3",
        "",
      ].join("\n"),
    );
  });
});
