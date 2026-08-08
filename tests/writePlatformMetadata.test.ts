import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  EXPECTED_TARGETS,
  buildBuildMetadata,
  type ExpectedTarget,
  type PlatformBuildMetadataRecord,
  type ReleaseIdentity,
} from "../scripts/release/release-contract.mjs";

const repositoryRoot = path.resolve(__dirname, "..");
const writerPath = path.join(
  repositoryRoot,
  "scripts",
  "release",
  "write-platform-metadata.mjs",
);
const sourceSha = "b".repeat(40);
const temporaryRoots: string[] = [];
const containerInputNames = [
  "CONTAINER_IMAGE_REFERENCE",
  "CONTAINER_MANIFEST_DIGEST",
  "ACTUAL_CONTAINER_OS_ID",
  "ACTUAL_CONTAINER_OS_VERSION_ID",
  "ACTUAL_CONTAINER_UNAME_MACHINE",
] as const;

function temporaryDirectory(): string {
  const root = mkdtempSync(path.join(tmpdir(), "fyagent-platform-metadata-"));
  temporaryRoots.push(root);
  return root;
}

function releaseIdentity(mode: "preflight" | "formal"): ReleaseIdentity {
  return {
    productVersion: "0.3.0",
    tag: "v0.3.0",
    sourceSha,
    repository: "NongHua123/fyagent",
    repositoryId: "1313497021",
    workflowPath: ".github/workflows/release.yml",
    workflowRef:
      mode === "formal"
        ? "NongHua123/fyagent/.github/workflows/release.yml@refs/tags/v0.3.0"
        : "NongHua123/fyagent/.github/workflows/release.yml@refs/heads/main",
    workflowSha: sourceSha,
    runId: "123456",
    runAttempt: "2",
    event: mode === "formal" ? "push" : "workflow_dispatch",
    mode,
    ciWorkflowPath: mode === "formal" ? ".github/workflows/ci.yml" : null,
    ciRunId: mode === "formal" ? "987654" : null,
    ciRunAttempt: mode === "formal" ? "3" : null,
  };
}

function writerEnvironment(
  expected: ExpectedTarget,
  mode: "preflight" | "formal" = "preflight",
): NodeJS.ProcessEnv {
  const identity = releaseIdentity(mode);
  const environment: NodeJS.ProcessEnv = {
    TARGET_GROUP: expected.targetGroup,
    TARGET_PLATFORM: expected.platform,
    TARGET_ARCHITECTURE: expected.architecture,
    REQUESTED_RUNNER_LABEL: expected.requestedRunnerLabel,
    ACTUAL_RUNNER_OS: expected.expectedRunnerOs,
    ACTUAL_RUNNER_ARCH: expected.expectedRunnerArch,
    ACTUAL_NODE_VERSION: "v24.19.0",
    ACTUAL_PNPM_VERSION: "10.12.3",
    ACTUAL_RUST_VERSION: "rustc 1.97.1 (reviewed 2026-08-08)",
    APP_VERSION: identity.productVersion,
    RELEASE_TAG: identity.tag,
    SOURCE_SHA: identity.sourceSha,
    GITHUB_REPOSITORY: identity.repository,
    GITHUB_REPOSITORY_ID: identity.repositoryId,
    GITHUB_WORKFLOW_REF: identity.workflowRef,
    GITHUB_WORKFLOW_SHA: identity.workflowSha,
    GITHUB_RUN_ID: identity.runId,
    GITHUB_RUN_ATTEMPT: identity.runAttempt,
    GITHUB_EVENT_NAME: identity.event,
    RELEASE_MODE: mode,
  };
  if (mode === "formal") {
    environment.EXPECTED_CI_RUN_ID = identity.ciRunId!;
    environment.EXPECTED_CI_RUN_ATTEMPT = identity.ciRunAttempt!;
  }
  if (expected.expectedContainer !== null) {
    environment.CONTAINER_IMAGE_REFERENCE =
      expected.expectedContainer.imageReference;
    environment.CONTAINER_MANIFEST_DIGEST =
      expected.expectedContainer.manifestDigest;
    environment.ACTUAL_CONTAINER_OS_ID = expected.expectedContainer.osReleaseId;
    environment.ACTUAL_CONTAINER_OS_VERSION_ID =
      expected.expectedContainer.osReleaseVersionId;
    environment.ACTUAL_CONTAINER_UNAME_MACHINE =
      expected.expectedContainer.unameMachine;
  }
  return environment;
}

function expectedRecord(
  expected: ExpectedTarget,
  mode: "preflight" | "formal" = "preflight",
): PlatformBuildMetadataRecord {
  return {
    schema: "fyagent-platform-build/v1",
    targetGroup: expected.targetGroup,
    platform: expected.platform,
    architecture: expected.architecture,
    runner: {
      requestedLabel: expected.requestedRunnerLabel,
      context: {
        os: expected.expectedRunnerOs,
        arch: expected.expectedRunnerArch,
      },
    },
    container:
      expected.expectedContainer === null
        ? null
        : {
            configuredImage: {
              reference: expected.expectedContainer.imageReference,
              manifestDigest: expected.expectedContainer.manifestDigest,
            },
            observed: {
              osRelease: {
                id: expected.expectedContainer.osReleaseId,
                versionId: expected.expectedContainer.osReleaseVersionId,
              },
              unameMachine: expected.expectedContainer.unameMachine,
            },
          },
    toolchain: {
      node: "v24.19.0",
      pnpm: "10.12.3",
      rustc: "rustc 1.97.1 (reviewed 2026-08-08)",
    },
    identity: releaseIdentity(mode),
  };
}

function invokeWriter(
  expected: ExpectedTarget,
  options: {
    mode?: "preflight" | "formal";
    mutateEnvironment?: (environment: NodeJS.ProcessEnv) => void;
    outputPath?: string;
  } = {},
) {
  const mode = options.mode ?? "preflight";
  const outputPath =
    options.outputPath ?? path.join(temporaryDirectory(), "platform.json");
  const environment = writerEnvironment(expected, mode);
  options.mutateEnvironment?.(environment);
  const result = spawnSync(process.execPath, [writerPath, outputPath], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: environment,
  });
  return { environment, outputPath, result };
}

function expectWriterFailure(
  expected: ExpectedTarget,
  mutateEnvironment: (environment: NodeJS.ProcessEnv) => void,
  error: RegExp,
  mode: "preflight" | "formal" = "preflight",
): void {
  const { result } = invokeWriter(expected, { mode, mutateEnvironment });
  expect(result.status).not.toBe(0);
  expect(result.stderr).toMatch(error);
}

function setEnvironmentVariable(
  name: string,
  value: string,
): (environment: NodeJS.ProcessEnv) => void {
  return (environment) => {
    environment[name] = value;
  };
}

function allObjectKeys(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap(allObjectKeys);
  if (value === null || typeof value !== "object") return [];
  return Object.entries(value).flatMap(([key, nested]) => [
    key,
    ...allObjectKeys(nested),
  ]);
}

afterEach(() => {
  while (temporaryRoots.length > 0) {
    rmSync(temporaryRoots.pop()!, { force: true, recursive: true });
  }
});

describe("write-platform-metadata CLI", () => {
  for (const expected of EXPECTED_TARGETS) {
    it(`writes the exact source-explicit ${expected.targetGroup} record`, () => {
      const { outputPath, result } = invokeWriter(expected);
      expect(result.status, result.stderr).toBe(0);
      expect(JSON.parse(readFileSync(outputPath, "utf8"))).toEqual(
        expectedRecord(expected),
      );
    });
  }

  for (const variable of ["ACTUAL_RUNNER_OS", "ACTUAL_RUNNER_ARCH"] as const) {
    it(`rejects missing ${variable}`, () => {
      expectWriterFailure(
        EXPECTED_TARGETS[3],
        (environment) => delete environment[variable],
        new RegExp(variable),
      );
    });

    it(`rejects blank ${variable}`, () => {
      expectWriterFailure(
        EXPECTED_TARGETS[3],
        (environment) => (environment[variable] = "   "),
        new RegExp(variable),
      );
    });
  }

  for (const variable of containerInputNames) {
    it(`rejects missing Linux ${variable}`, () => {
      expectWriterFailure(
        EXPECTED_TARGETS[3],
        (environment) => delete environment[variable],
        new RegExp(variable),
      );
    });

    it(`rejects blank Linux ${variable}`, () => {
      expectWriterFailure(
        EXPECTED_TARGETS[3],
        (environment) => (environment[variable] = " \t "),
        new RegExp(variable),
      );
    });
  }

  for (const expected of [EXPECTED_TARGETS[0], EXPECTED_TARGETS[1]]) {
    it(`rejects partial container evidence for ${expected.targetGroup}`, () => {
      expectWriterFailure(
        expected,
        (environment) =>
          (environment.CONTAINER_MANIFEST_DIGEST = `sha256:${"0".repeat(64)}`),
        /must not supply container metadata inputs.*CONTAINER_MANIFEST_DIGEST/,
      );
    });
  }

  it.each([
    [
      "unknown target",
      setEnvironmentVariable("TARGET_GROUP", "linux-unknown"),
      /Unsupported target group/,
    ],
    [
      "platform contradiction",
      setEnvironmentVariable("TARGET_PLATFORM", "windows"),
      /TARGET_PLATFORM/,
    ],
    [
      "architecture contradiction",
      setEnvironmentVariable("TARGET_ARCHITECTURE", "arm64"),
      /TARGET_ARCHITECTURE/,
    ],
    [
      "requested-label contradiction",
      setEnvironmentVariable("REQUESTED_RUNNER_LABEL", "ubuntu-24.04-arm"),
      /REQUESTED_RUNNER_LABEL/,
    ],
    [
      "runner OS contradiction",
      setEnvironmentVariable("ACTUAL_RUNNER_OS", "Windows"),
      /ACTUAL_RUNNER_OS/,
    ],
    [
      "runner architecture contradiction",
      setEnvironmentVariable("ACTUAL_RUNNER_ARCH", "ARM64"),
      /ACTUAL_RUNNER_ARCH/,
    ],
    [
      "undocumented runner architecture",
      setEnvironmentVariable("ACTUAL_RUNNER_ARCH", "UNIVERSAL"),
      /documented GitHub runner architecture/,
    ],
    [
      "OS ID contradiction",
      setEnvironmentVariable("ACTUAL_CONTAINER_OS_ID", "debian"),
      /ACTUAL_CONTAINER_OS_ID/,
    ],
    [
      "OS version contradiction",
      setEnvironmentVariable("ACTUAL_CONTAINER_OS_VERSION_ID", "24.04"),
      /ACTUAL_CONTAINER_OS_VERSION_ID/,
    ],
    [
      "uname contradiction",
      setEnvironmentVariable("ACTUAL_CONTAINER_UNAME_MACHINE", "aarch64"),
      /ACTUAL_CONTAINER_UNAME_MACHINE/,
    ],
  ] as const)("rejects %s", (_label, mutateEnvironment, error) => {
    expectWriterFailure(EXPECTED_TARGETS[3], mutateEnvironment, error);
  });

  it.each([
    ["macos-universal", EXPECTED_TARGETS[0], "X64", /ACTUAL_RUNNER_ARCH/],
    ["linux-x64", EXPECTED_TARGETS[3], "ARM64", /ACTUAL_RUNNER_ARCH/],
    ["linux-arm64", EXPECTED_TARGETS[4], "X64", /ACTUAL_RUNNER_ARCH/],
  ])(
    "rejects %s paired with the opposite runner context",
    (_label, expected, runnerArch, error) => {
      expectWriterFailure(
        expected,
        (environment) => (environment.ACTUAL_RUNNER_ARCH = runnerArch),
        error,
      );
    },
  );

  it.each([
    [
      "linux-x64",
      EXPECTED_TARGETS[3],
      "aarch64",
      /ACTUAL_CONTAINER_UNAME_MACHINE/,
    ],
    [
      "linux-arm64",
      EXPECTED_TARGETS[4],
      "x86_64",
      /ACTUAL_CONTAINER_UNAME_MACHINE/,
    ],
  ])(
    "rejects %s paired with the opposite uname observation",
    (_label, expected, unameMachine, error) => {
      expectWriterFailure(
        expected,
        (environment) =>
          (environment.ACTUAL_CONTAINER_UNAME_MACHINE = unameMachine),
        error,
      );
    },
  );

  it.each([
    ["short digest", "sha256:abc"],
    ["uppercase digest", `sha256:${"A".repeat(64)}`],
    ["wrong algorithm", `sha512:${"0".repeat(64)}`],
  ])("rejects malformed %s", (_label, digest) => {
    expectWriterFailure(
      EXPECTED_TARGETS[3],
      (environment) => (environment.CONTAINER_MANIFEST_DIGEST = digest),
      /CONTAINER_MANIFEST_DIGEST must be a lowercase SHA-256 digest/,
    );
  });

  it("rejects an image-reference suffix inconsistent with its digest", () => {
    expectWriterFailure(
      EXPECTED_TARGETS[3],
      (environment) =>
        (environment.CONTAINER_IMAGE_REFERENCE = `docker.io/library/ubuntu:22.04@sha256:${"0".repeat(64)}`),
      /CONTAINER_IMAGE_REFERENCE must end with CONTAINER_MANIFEST_DIGEST/,
    );
  });

  it.each([
    ["linux-x64", EXPECTED_TARGETS[3], EXPECTED_TARGETS[4]],
    ["linux-arm64", EXPECTED_TARGETS[4], EXPECTED_TARGETS[3]],
  ])(
    "rejects %s paired with the opposite Linux container mapping",
    (_label, expected, opposite) => {
      expectWriterFailure(
        expected,
        (environment) => {
          environment.CONTAINER_IMAGE_REFERENCE =
            opposite.expectedContainer!.imageReference;
          environment.CONTAINER_MANIFEST_DIGEST =
            opposite.expectedContainer!.manifestDigest;
        },
        /CONTAINER_IMAGE_REFERENCE|CONTAINER_MANIFEST_DIGEST/,
      );
    },
  );

  it("preserves formal Required-CI identity", () => {
    const expected = EXPECTED_TARGETS[1];
    const { outputPath, result } = invokeWriter(expected, { mode: "formal" });
    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(readFileSync(outputPath, "utf8"))).toEqual(
      expectedRecord(expected, "formal"),
    );
  });

  it.each(["EXPECTED_CI_RUN_ID", "EXPECTED_CI_RUN_ATTEMPT"])(
    "rejects formal metadata missing %s",
    (variable) => {
      expectWriterFailure(
        EXPECTED_TARGETS[1],
        (environment) => delete environment[variable],
        /requires the bound Required CI attempt/,
        "formal",
      );
    },
  );

  it("rejects a Required-CI claim in preflight metadata", () => {
    expectWriterFailure(
      EXPECTED_TARGETS[1],
      (environment) => {
        environment.EXPECTED_CI_RUN_ID = "987654";
        environment.EXPECTED_CI_RUN_ATTEMPT = "3";
      },
      /must not claim a Required CI binding/,
    );
  });

  it("does not replace an existing output file", () => {
    const root = temporaryDirectory();
    const outputPath = path.join(root, "platform.json");
    writeFileSync(outputPath, "preserve-me\n");
    const { result } = invokeWriter(EXPECTED_TARGETS[1], { outputPath });
    expect(result.status).not.toBe(0);
    expect(result.stderr).toMatch(/EEXIST|file already exists/i);
    expect(readFileSync(outputPath, "utf8")).toBe("preserve-me\n");
  });

  it("ignores hostile ambient hosted-runner variables", () => {
    const expected = EXPECTED_TARGETS[3];
    const { outputPath, result } = invokeWriter(expected, {
      mutateEnvironment: (environment) => {
        environment.RUNNER_OS = "Windows";
        environment.RUNNER_ARCH = "ARM64";
        environment.ImageOS = "host-poison";
        environment.ImageVersion = "host-version-poison";
      },
    });
    expect(result.status, result.stderr).toBe(0);
    const record = JSON.parse(readFileSync(outputPath, "utf8")) as unknown;
    expect(record).toEqual(expectedRecord(expected));
    expect(allObjectKeys(record)).not.toEqual(
      expect.arrayContaining([
        "imageOs",
        "imageVersion",
        "ImageOS",
        "ImageVersion",
      ]),
    );
    expect(JSON.stringify(record)).not.toContain("poison");
  });

  it("feeds all five writer records into the canonical aggregate", () => {
    const metadataDirectory = temporaryDirectory();
    for (const expected of EXPECTED_TARGETS) {
      const outputPath = path.join(
        metadataDirectory,
        `${expected.targetGroup}.json`,
      );
      const { result } = invokeWriter(expected, { outputPath });
      expect(result.status, result.stderr).toBe(0);
    }

    const metadata = buildBuildMetadata({
      metadataDirectory,
      identity: releaseIdentity("preflight"),
      generatedAt: "2026-08-08T00:00:00.000Z",
    });
    expect(metadata.schema).toBe("fyagent-build-metadata/v1");
    expect(metadata.targets).toHaveLength(5);
    expect(metadata.targets).toEqual(
      EXPECTED_TARGETS.map((expected) => {
        const { identity: _identity, ...target } = expectedRecord(expected);
        return target;
      }),
    );
  });
});
