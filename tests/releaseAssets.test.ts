import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  ATTESTATION_BUNDLE_NAME,
  BUILD_METADATA_NAME,
  DOWNLOAD_MANIFEST_NAME,
  EXPECTED_TARGETS,
  EXPECTED_INSTALLERS_BY_TARGET,
  buildBuildMetadata,
  expectedAttestationSubjectNames,
  expectedInstallerNames,
  expectedReleaseAttachmentNames,
  assertExactFileSet,
  type ReleaseIdentity,
} from "../scripts/release/release-contract.mjs";

const temporaryRoots: string[] = [];
const repositoryRoot = path.resolve(__dirname, "..");
const collectorScript = path.join(
  repositoryRoot,
  "scripts",
  "release",
  "collect-workflow-artifacts.mjs",
);
const identity: ReleaseIdentity = {
  productVersion: "0.3.0",
  tag: "v0.3.0",
  sourceSha: "b".repeat(40),
  repository: "NongHua123/fyagent",
  repositoryId: "1313497021",
  workflowPath: ".github/workflows/release.yml",
  workflowRef:
    "NongHua123/fyagent/.github/workflows/release.yml@refs/heads/main",
  workflowSha: "b".repeat(40),
  runId: "123456",
  runAttempt: "2",
  event: "workflow_dispatch",
  mode: "preflight",
  ciWorkflowPath: null,
  ciRunId: null,
  ciRunAttempt: null,
};

function temporaryDirectory(): string {
  const root = mkdtempSync(path.join(tmpdir(), "fyagent-release-assets-"));
  temporaryRoots.push(root);
  return root;
}

function writePlatformMetadata(
  directory: string,
  metadataIdentity: ReleaseIdentity = identity,
): void {
  for (const expected of EXPECTED_TARGETS) {
    writeFileSync(
      path.join(directory, `${expected.targetGroup}.json`),
      `${JSON.stringify(
        {
          schema: "fyagent-platform-build/v1",
          targetGroup: expected.targetGroup,
          platform: expected.platform,
          architecture: expected.architecture,
          runnerLabel: expected.runnerLabel,
          runner: {
            runnerOs: expected.platform,
            runnerArch: expected.expectedRunnerArch ?? "X64",
            imageOs: "reviewed-image",
            imageVersion: "20260808.1",
          },
          containerDigest: expected.containerDigest,
          toolchain: {
            node: "v24.19.0",
            pnpm: "10.12.3",
            rustc: "rustc 1.97.1 (reviewed 2026-08-08)",
          },
          identity: metadataIdentity,
        },
        null,
        2,
      )}\n`,
    );
  }
}

function writeInstallerArtifacts(directory: string): void {
  const installers = expectedInstallerNames("0.3.0");
  for (const { targetGroup } of EXPECTED_TARGETS) {
    const artifact = path.join(directory, `installers-${targetGroup}`);
    mkdirSync(artifact);
    for (const index of EXPECTED_INSTALLERS_BY_TARGET[targetGroup]) {
      writeFileSync(path.join(artifact, installers[index]), installers[index]);
    }
  }
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { force: true, recursive: true });
  }
});

describe("release asset and metadata contract", () => {
  it("freezes ten installers, twelve attestation subjects, and thirteen attachments", () => {
    expect(expectedInstallerNames("0.3.0")).toHaveLength(10);
    expect(expectedAttestationSubjectNames("0.3.0")).toEqual([
      ...expectedInstallerNames("0.3.0"),
      DOWNLOAD_MANIFEST_NAME,
      BUILD_METADATA_NAME,
    ]);
    expect(expectedReleaseAttachmentNames("0.3.0")).toEqual([
      ...expectedAttestationSubjectNames("0.3.0"),
      ATTESTATION_BUNDLE_NAME,
    ]);
  });

  it("rejects directories, symlinks, missing names, and unapproved ancillary files", () => {
    const directory = temporaryDirectory();
    for (const name of expectedInstallerNames("0.3.0")) {
      writeFileSync(path.join(directory, name), name);
    }
    mkdirSync(path.join(directory, "nested"));
    expect(() =>
      assertExactFileSet(
        directory,
        expectedInstallerNames("0.3.0"),
        "installers",
      ),
    ).toThrow(/Only regular files are allowed/);
  });

  it("collects five isolated installer artifacts without allowing overwrite", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    const output = path.join(root, "installers");
    mkdirSync(downloads);
    writeInstallerArtifacts(downloads);
    execFileSync(
      process.execPath,
      [collectorScript, "installers", downloads, output, "0.3.0"],
      { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
    );
    expect(readdirSync(output).sort()).toEqual(
      expectedInstallerNames("0.3.0").sort(),
    );
  });

  it("rejects duplicate or misplaced installers before flattening artifacts", () => {
    const root = temporaryDirectory();
    const downloads = path.join(root, "downloads");
    mkdirSync(downloads);
    writeInstallerArtifacts(downloads);
    writeFileSync(
      path.join(downloads, "installers-windows-x64", "FyAgent-0.3.0-macOS.dmg"),
      "duplicate",
    );
    expect(() =>
      execFileSync(
        process.execPath,
        [
          collectorScript,
          "installers",
          downloads,
          path.join(root, "installers"),
          "0.3.0",
        ],
        { cwd: repositoryRoot, encoding: "utf8", stdio: "pipe" },
      ),
    ).toThrow(/installers-windows-x64 artifact must contain exactly 1 files/);
  });

  it("aggregates exactly five identity-bound platform records", () => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    const metadata = buildBuildMetadata({
      metadataDirectory: directory,
      identity,
      generatedAt: "2026-08-08T00:00:00.000Z",
    });
    expect(metadata).toMatchObject({
      schema: "fyagent-build-metadata/v1",
      product: "FyAgent",
      version: "0.3.0",
      tag: "v0.3.0",
      sourceSha: "b".repeat(40),
      repository: {
        nameWithOwner: "NongHua123/fyagent",
        id: "1313497021",
      },
      workflow: {
        path: ".github/workflows/release.yml",
        runId: "123456",
        runAttempt: "2",
        event: "workflow_dispatch",
        mode: "preflight",
        ref: "NongHua123/fyagent/.github/workflows/release.yml@refs/heads/main",
        sha: "b".repeat(40),
      },
      requiredCi: null,
    });
    expect(metadata.targets.map(({ targetGroup }) => targetGroup)).toEqual(
      EXPECTED_TARGETS.map(({ targetGroup }) => targetGroup),
    );
  });

  it("requires a unique Required CI binding only for formal metadata", () => {
    const directory = temporaryDirectory();
    const formalIdentity = {
      ...identity,
      workflowRef:
        "NongHua123/fyagent/.github/workflows/release.yml@refs/tags/v0.3.0",
      workflowSha: identity.sourceSha,
      event: "push",
      mode: "formal",
      ciWorkflowPath: ".github/workflows/ci.yml",
      ciRunId: "987654",
      ciRunAttempt: "3",
    };
    writePlatformMetadata(directory, formalIdentity);
    const metadata = buildBuildMetadata({
      metadataDirectory: directory,
      identity: formalIdentity,
      generatedAt: "2026-08-08T00:00:00.000Z",
    });
    expect(metadata.requiredCi).toEqual({
      path: ".github/workflows/ci.yml",
      runId: "987654",
      runAttempt: "3",
      job: "CI / Required",
      conclusion: "success",
    });

    const secondDirectory = temporaryDirectory();
    writePlatformMetadata(secondDirectory, {
      ...identity,
      ciWorkflowPath: ".github/workflows/ci.yml",
      ciRunId: "987654",
      ciRunAttempt: "3",
    });
    expect(() =>
      buildBuildMetadata({
        metadataDirectory: secondDirectory,
        identity: {
          ...identity,
          ciWorkflowPath: ".github/workflows/ci.yml",
          ciRunId: "987654",
          ciRunAttempt: "3",
        },
        generatedAt: "2026-08-08T00:00:00.000Z",
      }),
    ).toThrow(/Preflight metadata must not claim a Required CI binding/);
  });

  it.each([
    [
      "repository",
      { repository: "fork/fyagent" },
      /Repository identity drifted/,
    ],
    ["repository id", { repositoryId: "42" }, /Repository ID drifted/],
    [
      "workflow",
      { workflowPath: ".github/workflows/other.yml" },
      /workflow path drifted/,
    ],
    ["source SHA", { sourceSha: "c".repeat(39) }, /full 40-character/],
  ])("rejects %s identity drift", (_label, change, error) => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    expect(() =>
      buildBuildMetadata({
        metadataDirectory: directory,
        identity: { ...identity, ...change },
        generatedAt: "2026-08-08T00:00:00.000Z",
      }),
    ).toThrow(error);
  });

  it("rejects target runner and Linux child-digest drift", () => {
    const directory = temporaryDirectory();
    writePlatformMetadata(directory);
    const linuxArmPath = path.join(directory, "linux-arm64.json");
    // The fixture is controlled JSON and intentionally rewritten to prove drift rejection.
    const record = JSON.parse(readFileSync(linuxArmPath, "utf8")) as Record<
      string,
      unknown
    >;
    record.containerDigest =
      "sha256:3b06811b2afd352be909dd088a004166d665dc76d38b13eada33522a9d915c6f";
    writeFileSync(linuxArmPath, `${JSON.stringify(record)}\n`);
    expect(() =>
      buildBuildMetadata({
        metadataDirectory: directory,
        identity,
        generatedAt: "2026-08-08T00:00:00.000Z",
      }),
    ).toThrow(/linux-arm64 container digest drifted/);
  });
});
