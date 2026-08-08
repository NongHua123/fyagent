import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  DOWNLOAD_MANIFEST_NAME,
  expectedInstallerNames,
} from "../scripts/release/release-contract.mjs";

const repositoryRoot = path.resolve(__dirname, "..");
const manifestScript = path.join(
  repositoryRoot,
  "scripts",
  "generate-download-manifest.mjs",
);
const temporaryRoots: string[] = [];
const version = "0.3.0";
const tag = "v0.3.0";
const sourceSha = "a".repeat(40);
const publishedAt = "2026-08-08T00:00:00.000Z";

function createAssetsDirectory(): string {
  const root = mkdtempSync(path.join(tmpdir(), "fyagent-download-manifest-"));
  temporaryRoots.push(root);
  return root;
}

function populateExactInstallers(directory: string): void {
  for (const name of expectedInstallerNames(version)) {
    writeFileSync(path.join(directory, name), `contents:${name}`);
  }
}

function runGenerator(
  assetsDirectory: string,
  overrides: string[] = [],
): string {
  const manifestPath = path.join(assetsDirectory, DOWNLOAD_MANIFEST_NAME);
  execFileSync(
    process.execPath,
    [
      manifestScript,
      assetsDirectory,
      version,
      tag,
      sourceSha,
      "https://github.com/NongHua123/fyagent/releases/download",
      publishedAt,
      manifestPath,
      ...overrides,
    ],
    { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
  );
  return manifestPath;
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { force: true, recursive: true });
  }
});

describe("release download manifest", () => {
  it("records the exact ten installers with frozen identity and streaming digests", () => {
    const assetsDirectory = createAssetsDirectory();
    populateExactInstallers(assetsDirectory);
    const manifestPath = runGenerator(assetsDirectory);
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
      schema: string;
      product: string;
      version: string;
      tag: string;
      sourceSha: string;
      publishedAt: string;
      assets: Array<{
        architecture: string;
        format: string;
        name: string;
        sha256: string;
        sizeBytes: number;
        url: string;
      }>;
    };

    expect(manifest).toMatchObject({
      schema: "fyagent-download-manifest/v2",
      product: "FyAgent",
      version,
      tag,
      sourceSha,
      publishedAt,
    });
    expect(manifest.assets.map(({ name }) => name)).toEqual(
      expectedInstallerNames(version),
    );
    expect(manifest.assets).toHaveLength(10);
    const windowsArm64 = manifest.assets.find(
      ({ name }) => name === "FyAgent-0.3.0-Windows-arm64.msi",
    );
    const expectedContents = "contents:FyAgent-0.3.0-Windows-arm64.msi";
    expect(windowsArm64).toMatchObject({
      architecture: "arm64",
      format: "msi",
      sha256: createHash("sha256").update(expectedContents).digest("hex"),
      sizeBytes: Buffer.byteLength(expectedContents),
      url: "https://github.com/NongHua123/fyagent/releases/download/v0.3.0/FyAgent-0.3.0-Windows-arm64.msi",
    });
  });

  it.each([
    [
      "missing",
      (directory: string) =>
        rmSync(path.join(directory, expectedInstallerNames(version)[0])),
    ],
    [
      "extra",
      (directory: string) =>
        writeFileSync(path.join(directory, "unexpected.sig"), "extra"),
    ],
  ])("rejects a %s installer-set member", (_label, mutate) => {
    const assetsDirectory = createAssetsDirectory();
    populateExactInstallers(assetsDirectory);
    mutate(assetsDirectory);
    expect(() => runGenerator(assetsDirectory)).toThrow(
      /installer directory must contain exactly 10 files/,
    );
  });

  it("rejects empty installers before evidence generation", () => {
    const assetsDirectory = createAssetsDirectory();
    populateExactInstallers(assetsDirectory);
    writeFileSync(
      path.join(assetsDirectory, expectedInstallerNames(version)[3]),
      "",
    );
    expect(() => runGenerator(assetsDirectory)).toThrow(
      /Release evidence files must not be empty/,
    );
  });

  it.each([
    ["v0.3.1", sourceSha, /Release tag must exactly match v0\.3\.0/],
    [tag, "A".repeat(40), /lowercase full 40-character Git commit SHA/],
    [tag, "a".repeat(39), /lowercase full 40-character Git commit SHA/],
  ])(
    "rejects invalid tag or SHA identity",
    (candidateTag, candidateSha, error) => {
      const assetsDirectory = createAssetsDirectory();
      populateExactInstallers(assetsDirectory);
      expect(() =>
        execFileSync(
          process.execPath,
          [
            manifestScript,
            assetsDirectory,
            version,
            candidateTag,
            candidateSha,
            "https://github.com/NongHua123/fyagent/releases/download",
            publishedAt,
          ],
          { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
        ),
      ).toThrow(error);
    },
  );
});
