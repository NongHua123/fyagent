import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(__dirname, "..");
const manifestScript = path.join(
  repositoryRoot,
  "scripts",
  "generate-download-manifest.mjs",
);
const temporaryRoots: string[] = [];

function createAssetsDirectory(): string {
  const root = mkdtempSync(path.join(tmpdir(), "fyagent-download-manifest-"));
  temporaryRoots.push(root);
  return root;
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { force: true, recursive: true });
  }
});

describe("release download manifest", () => {
  it("records explicit version, tag, source SHA, assets, and post-build digests", () => {
    const assetsDirectory = createAssetsDirectory();
    const msiName = "FyAgent-0.2.1-Windows.msi";
    const dmgName = "FyAgent-0.2.1-macOS.dmg";
    const manifestPath = path.join(assetsDirectory, "download-manifest.json");
    writeFileSync(path.join(assetsDirectory, msiName), "windows-msi");
    writeFileSync(path.join(assetsDirectory, dmgName), "macos-dmg");

    execFileSync(
      process.execPath,
      [
        manifestScript,
        assetsDirectory,
        "0.2.1",
        "v0.2.1",
        "a".repeat(40),
        "https://github.com/example/fyagent/releases/download/",
        manifestPath,
      ],
      { cwd: repositoryRoot, encoding: "utf8" },
    );

    const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
      schema: string;
      version: string;
      tag: string;
      sourceSha: string;
      assets: Array<{
        arch: string;
        name: string;
        sha256: string;
        url: string;
      }>;
    };
    expect(manifest).toMatchObject({
      schema: "fyagent-download-manifest/v1",
      version: "0.2.1",
      tag: "v0.2.1",
      sourceSha: "a".repeat(40),
    });
    expect(manifest.assets).toEqual([
      expect.objectContaining({
        arch: "x64",
        name: msiName,
        sha256:
          "fa82c67c55f288f42c904461b40c2b429eff51054937a7455dc36b69c5f40cf8",
        url: `https://github.com/example/fyagent/releases/download/v0.2.1/${msiName}`,
      }),
      expect.objectContaining({
        arch: "universal",
        name: dmgName,
        sha256:
          "3a797b9bb92e3a0acb53fe3d1de290455763e8239f82f8096fcf8611c2481a09",
        url: `https://github.com/example/fyagent/releases/download/v0.2.1/${dmgName}`,
      }),
    ]);
  });

  it("rejects a tag that does not exactly name the supplied application version", () => {
    const assetsDirectory = createAssetsDirectory();
    writeFileSync(
      path.join(assetsDirectory, "FyAgent-0.2.1-Windows.msi"),
      "windows-msi",
    );

    expect(() =>
      execFileSync(
        process.execPath,
        [
          manifestScript,
          assetsDirectory,
          "0.2.1",
          "v0.2.2",
          "a".repeat(40),
          "https://github.com/example/fyagent/releases/download",
        ],
        { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
      ),
    ).toThrow(/Release tag must exactly match v0\.2\.1/);
  });

  it("rejects prerelease application versions outside the frozen release contract", () => {
    const assetsDirectory = createAssetsDirectory();
    writeFileSync(
      path.join(assetsDirectory, "FyAgent-0.2.1-rc.1-Windows.msi"),
      "windows-msi",
    );

    expect(() =>
      execFileSync(
        process.execPath,
        [
          manifestScript,
          assetsDirectory,
          "0.2.1-rc.1",
          "v0.2.1-rc.1",
          "a".repeat(40),
          "https://github.com/example/fyagent/releases/download",
        ],
        { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
      ),
    ).toThrow(/Invalid application version: 0\.2\.1-rc\.1/);
  });
});
