import { createHash } from "node:crypto";
import {
  createReadStream,
  lstatSync,
  readdirSync,
  readFileSync,
  statSync,
} from "node:fs";
import { basename, join } from "node:path";

export const PRODUCT_NAME = "FyAgent";
export const FORMAL_VERSION = "0.3.0";
export const FORMAL_TAG = `v${FORMAL_VERSION}`;
export const EXPECTED_REPOSITORY = "NongHua123/fyagent";
export const EXPECTED_REPOSITORY_ID = "1313497021";
export const RELEASE_WORKFLOW_PATH = ".github/workflows/release.yml";
export const CI_WORKFLOW_PATH = ".github/workflows/ci.yml";
export const DOWNLOAD_MANIFEST_NAME = "download-manifest.json";
export const BUILD_METADATA_NAME = "build-metadata.json";
export const ATTESTATION_BUNDLE_NAME = "artifact-attestation.sigstore.json";

const SHA_PATTERN = /^[0-9a-f]{40}$/;
const SHA256_DIGEST_PATTERN = /^sha256:[0-9a-f]{64}$/;
const STABLE_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export const GITHUB_RUNNER_ARCHITECTURES = Object.freeze([
  "X86",
  "X64",
  "ARM",
  "ARM64",
]);

export const INSTALLER_RULES = Object.freeze([
  {
    suffix: "-macOS.dmg",
    platform: "macos",
    kind: "dmg",
    architecture: "universal",
  },
  {
    suffix: "-macOS.zip",
    platform: "macos",
    kind: "zip",
    architecture: "universal",
  },
  {
    suffix: "-Windows.msi",
    platform: "windows",
    kind: "msi",
    architecture: "x64",
  },
  {
    suffix: "-Windows-arm64.msi",
    platform: "windows",
    kind: "msi",
    architecture: "arm64",
  },
  {
    suffix: "-Linux-x86_64.AppImage",
    platform: "linux",
    kind: "appimage",
    architecture: "x64",
  },
  {
    suffix: "-Linux-x86_64.deb",
    platform: "linux",
    kind: "deb",
    architecture: "x64",
  },
  {
    suffix: "-Linux-x86_64.rpm",
    platform: "linux",
    kind: "rpm",
    architecture: "x64",
  },
  {
    suffix: "-Linux-arm64.AppImage",
    platform: "linux",
    kind: "appimage",
    architecture: "arm64",
  },
  {
    suffix: "-Linux-arm64.deb",
    platform: "linux",
    kind: "deb",
    architecture: "arm64",
  },
  {
    suffix: "-Linux-arm64.rpm",
    platform: "linux",
    kind: "rpm",
    architecture: "arm64",
  },
]);

export const EXPECTED_TARGETS = Object.freeze([
  {
    targetGroup: "macos-universal",
    platform: "macos",
    architecture: "universal",
    requestedRunnerLabel: "macos-15",
    expectedRunnerOs: "macOS",
    expectedRunnerArch: "ARM64",
    expectedContainer: null,
  },
  {
    targetGroup: "windows-x64",
    platform: "windows",
    architecture: "x64",
    requestedRunnerLabel: "windows-2022",
    expectedRunnerOs: "Windows",
    expectedRunnerArch: "X64",
    expectedContainer: null,
  },
  {
    targetGroup: "windows-arm64",
    platform: "windows",
    architecture: "arm64",
    requestedRunnerLabel: "windows-11-arm",
    expectedRunnerOs: "Windows",
    expectedRunnerArch: "ARM64",
    expectedContainer: null,
  },
  {
    targetGroup: "linux-x64",
    platform: "linux",
    architecture: "x64",
    requestedRunnerLabel: "ubuntu-24.04",
    expectedRunnerOs: "Linux",
    expectedRunnerArch: "X64",
    expectedContainer: {
      imageReference:
        "docker.io/library/ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e",
      manifestDigest:
        "sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e",
      osReleaseId: "ubuntu",
      osReleaseVersionId: "22.04",
      unameMachine: "x86_64",
    },
  },
  {
    targetGroup: "linux-arm64",
    platform: "linux",
    architecture: "arm64",
    requestedRunnerLabel: "ubuntu-24.04-arm",
    expectedRunnerOs: "Linux",
    expectedRunnerArch: "ARM64",
    expectedContainer: {
      imageReference:
        "docker.io/library/ubuntu:22.04@sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149",
      manifestDigest:
        "sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149",
      osReleaseId: "ubuntu",
      osReleaseVersionId: "22.04",
      unameMachine: "aarch64",
    },
  },
]);

export const EXPECTED_INSTALLERS_BY_TARGET = Object.freeze({
  "macos-universal": Object.freeze([0, 1]),
  "windows-x64": Object.freeze([2]),
  "windows-arm64": Object.freeze([3]),
  "linux-x64": Object.freeze([4, 5, 6]),
  "linux-arm64": Object.freeze([7, 8, 9]),
});

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

export function assertReleaseIdentity({ version, tag, sourceSha }) {
  assert(
    STABLE_VERSION_PATTERN.test(version),
    `Invalid stable application version: ${version}`,
  );
  assert(
    tag === `v${version}`,
    `Release tag must exactly match v${version}; received ${tag}`,
  );
  assert(
    SHA_PATTERN.test(sourceSha),
    "source SHA must be a lowercase full 40-character Git commit SHA",
  );
}

export function expectedInstallerNames(version) {
  assert(
    STABLE_VERSION_PATTERN.test(version),
    `Invalid stable application version: ${version}`,
  );
  return INSTALLER_RULES.map(
    (rule) => `${PRODUCT_NAME}-${version}${rule.suffix}`,
  );
}

export function expectedAttestationSubjectNames(version) {
  return [
    ...expectedInstallerNames(version),
    DOWNLOAD_MANIFEST_NAME,
    BUILD_METADATA_NAME,
  ];
}

export function expectedReleaseAttachmentNames(version) {
  return [...expectedAttestationSubjectNames(version), ATTESTATION_BUNDLE_NAME];
}

function listFlatRegularFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).map((entry) => {
    assert(
      entry.isFile(),
      `Only regular files are allowed in ${directory}: ${entry.name}`,
    );
    const filePath = join(directory, entry.name);
    assert(
      !lstatSync(filePath).isSymbolicLink(),
      `Symbolic links are forbidden: ${entry.name}`,
    );
    assert(
      statSync(filePath).size > 0,
      `Release evidence files must not be empty: ${entry.name}`,
    );
    return entry.name;
  });
}

export function assertExactFileSet(directory, expectedNames, label) {
  const actual = listFlatRegularFiles(directory).sort();
  const expected = [...expectedNames].sort();
  assert(
    new Set(actual).size === actual.length,
    `${label} contains duplicate filenames`,
  );
  assert(
    actual.length === expected.length &&
      actual.every((name, index) => name === expected[index]),
    `${label} must contain exactly ${expected.length} files; expected ${expected.join(", ")}; received ${actual.join(", ")}`,
  );
  return expectedNames.map((name) => join(directory, name));
}

export function assertExactDirectorySet(directory, expectedNames, label) {
  const entries = readdirSync(directory, { withFileTypes: true });
  for (const entry of entries) {
    assert(
      entry.isDirectory(),
      `Only directories are allowed in ${directory}: ${entry.name}`,
    );
    assert(
      !lstatSync(join(directory, entry.name)).isSymbolicLink(),
      `Symbolic directory links are forbidden: ${entry.name}`,
    );
  }
  const actual = entries.map(({ name }) => name).sort();
  const expected = [...expectedNames].sort();
  assert(
    actual.length === expected.length &&
      actual.every((name, index) => name === expected[index]),
    `${label} must contain exactly ${expected.length} directories; expected ${expected.join(", ")}; received ${actual.join(", ")}`,
  );
}

export function assertExactInstallerSet(directory, version) {
  return assertExactFileSet(
    directory,
    expectedInstallerNames(version),
    "installer directory",
  );
}

export async function sha256File(filePath) {
  const hash = createHash("sha256");
  await new Promise((resolve, reject) => {
    const stream = createReadStream(filePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("error", reject);
    stream.on("end", resolve);
  });
  return hash.digest("hex");
}

export async function buildDownloadManifest({
  assetsDirectory,
  version,
  tag,
  sourceSha,
  baseUrl,
  publishedAt,
}) {
  assertReleaseIdentity({ version, tag, sourceSha });
  assert(
    baseUrl && URL.canParse(baseUrl),
    `Invalid release base URL: ${baseUrl}`,
  );
  assert(
    typeof publishedAt === "string" &&
      !Number.isNaN(Date.parse(publishedAt)) &&
      new Date(publishedAt).toISOString() === publishedAt,
    `publishedAt must be an ISO-8601 instant: ${publishedAt}`,
  );

  const paths = assertExactInstallerSet(assetsDirectory, version);
  const normalizedBase = baseUrl.replace(/\/+$/, "");
  const assets = [];
  for (let index = 0; index < paths.length; index += 1) {
    const filePath = paths[index];
    const rule = INSTALLER_RULES[index];
    const name = basename(filePath);
    const sizeBytes = statSync(filePath).size;
    assert(sizeBytes > 0, `Release installer must not be empty: ${name}`);
    assets.push({
      name,
      platform: rule.platform,
      architecture: rule.architecture,
      format: rule.kind,
      sizeBytes,
      sha256: await sha256File(filePath),
      url: `${normalizedBase}/${tag}/${encodeURIComponent(name)}`,
    });
  }

  return {
    schema: "fyagent-download-manifest/v2",
    product: PRODUCT_NAME,
    version,
    tag,
    sourceSha,
    publishedAt,
    assets,
  };
}

function readJson(filePath) {
  try {
    return JSON.parse(readFileSync(filePath, "utf8"));
  } catch (error) {
    throw new Error(`Invalid JSON in ${filePath}: ${error.message}`);
  }
}

function requireNonEmptyString(value, label) {
  assert(
    typeof value === "string" && value.trim() !== "",
    `${label} must be a non-empty string`,
  );
}

const PLATFORM_METADATA_KEYS = Object.freeze([
  "schema",
  "targetGroup",
  "platform",
  "architecture",
  "runner",
  "container",
  "toolchain",
  "identity",
]);
const RUNNER_KEYS = Object.freeze(["requestedLabel", "context"]);
const RUNNER_CONTEXT_KEYS = Object.freeze(["os", "arch"]);
const CONTAINER_KEYS = Object.freeze(["configuredImage", "observed"]);
const CONFIGURED_IMAGE_KEYS = Object.freeze(["reference", "manifestDigest"]);
const CONTAINER_OBSERVED_KEYS = Object.freeze(["osRelease", "unameMachine"]);
const OS_RELEASE_KEYS = Object.freeze(["id", "versionId"]);
const TOOLCHAIN_KEYS = Object.freeze(["node", "pnpm", "rustc"]);
const IDENTITY_KEYS = Object.freeze([
  "productVersion",
  "tag",
  "sourceSha",
  "repository",
  "repositoryId",
  "workflowPath",
  "workflowRef",
  "workflowSha",
  "runId",
  "runAttempt",
  "event",
  "mode",
  "ciWorkflowPath",
  "ciRunId",
  "ciRunAttempt",
]);

function assertExactKeys(value, expectedKeys, label) {
  assert(
    value !== null && typeof value === "object" && !Array.isArray(value),
    `${label} must be an object`,
  );
  const actualKeys = Object.keys(value).sort();
  const sortedExpectedKeys = [...expectedKeys].sort();
  assert(
    actualKeys.length === sortedExpectedKeys.length &&
      actualKeys.every((key, index) => key === sortedExpectedKeys[index]),
    `${label} must contain exactly these keys: ${sortedExpectedKeys.join(", ")}; received ${actualKeys.join(", ")}`,
  );
}

function validatePlatformMetadata(metadata, expected, identity) {
  assertExactKeys(
    metadata,
    PLATFORM_METADATA_KEYS,
    `${expected.targetGroup} platform metadata`,
  );
  assert(
    metadata.schema === "fyagent-platform-build/v1",
    `Invalid platform metadata schema for ${expected.targetGroup}`,
  );
  for (const key of ["targetGroup", "platform", "architecture"]) {
    assert(
      metadata[key] === expected[key],
      `${expected.targetGroup} ${key} must be ${expected[key]}; received ${metadata[key]}`,
    );
  }

  assertExactKeys(
    metadata.runner,
    RUNNER_KEYS,
    `${expected.targetGroup} runner`,
  );
  assert(
    metadata.runner.requestedLabel === expected.requestedRunnerLabel,
    `${expected.targetGroup} requested runner label drifted`,
  );
  assertExactKeys(
    metadata.runner.context,
    RUNNER_CONTEXT_KEYS,
    `${expected.targetGroup} runner.context`,
  );
  requireNonEmptyString(
    metadata.runner.context.os,
    `${expected.targetGroup} runner.context.os`,
  );
  requireNonEmptyString(
    metadata.runner.context.arch,
    `${expected.targetGroup} runner.context.arch`,
  );
  assert(
    metadata.runner.context.os === expected.expectedRunnerOs,
    `${expected.targetGroup} runner context OS drifted`,
  );
  assert(
    GITHUB_RUNNER_ARCHITECTURES.includes(metadata.runner.context.arch),
    `${expected.targetGroup} runner context architecture is not a documented GitHub value`,
  );
  assert(
    metadata.runner.context.arch === expected.expectedRunnerArch,
    `${expected.targetGroup} runner context architecture drifted`,
  );

  if (expected.expectedContainer === null) {
    assert(
      metadata.container === null,
      `${expected.targetGroup} must record container as null`,
    );
  } else {
    assertExactKeys(
      metadata.container,
      CONTAINER_KEYS,
      `${expected.targetGroup} container`,
    );
    assertExactKeys(
      metadata.container.configuredImage,
      CONFIGURED_IMAGE_KEYS,
      `${expected.targetGroup} container.configuredImage`,
    );
    const { reference, manifestDigest } = metadata.container.configuredImage;
    requireNonEmptyString(
      reference,
      `${expected.targetGroup} container.configuredImage.reference`,
    );
    requireNonEmptyString(
      manifestDigest,
      `${expected.targetGroup} container.configuredImage.manifestDigest`,
    );
    assert(
      SHA256_DIGEST_PATTERN.test(manifestDigest),
      `${expected.targetGroup} container manifest digest must be lowercase SHA-256`,
    );
    assert(
      reference.endsWith(`@${manifestDigest}`),
      `${expected.targetGroup} container image reference must end with its manifest digest`,
    );
    assert(
      reference === expected.expectedContainer.imageReference,
      `${expected.targetGroup} configured container image reference drifted`,
    );
    assert(
      manifestDigest === expected.expectedContainer.manifestDigest,
      `${expected.targetGroup} container manifest digest drifted`,
    );

    assertExactKeys(
      metadata.container.observed,
      CONTAINER_OBSERVED_KEYS,
      `${expected.targetGroup} container.observed`,
    );
    assertExactKeys(
      metadata.container.observed.osRelease,
      OS_RELEASE_KEYS,
      `${expected.targetGroup} container.observed.osRelease`,
    );
    const { id, versionId } = metadata.container.observed.osRelease;
    const { unameMachine } = metadata.container.observed;
    requireNonEmptyString(
      id,
      `${expected.targetGroup} container.observed.osRelease.id`,
    );
    requireNonEmptyString(
      versionId,
      `${expected.targetGroup} container.observed.osRelease.versionId`,
    );
    requireNonEmptyString(
      unameMachine,
      `${expected.targetGroup} container.observed.unameMachine`,
    );
    assert(
      id === expected.expectedContainer.osReleaseId,
      `${expected.targetGroup} observed container OS ID drifted`,
    );
    assert(
      versionId === expected.expectedContainer.osReleaseVersionId,
      `${expected.targetGroup} observed container OS version drifted`,
    );
    assert(
      unameMachine === expected.expectedContainer.unameMachine,
      `${expected.targetGroup} observed container machine drifted`,
    );
  }

  assertExactKeys(
    metadata.identity,
    IDENTITY_KEYS,
    `${expected.targetGroup} identity`,
  );
  for (const key of IDENTITY_KEYS) {
    assert(
      metadata.identity?.[key] === identity[key],
      `${expected.targetGroup} identity ${key} drifted`,
    );
  }

  assertExactKeys(
    metadata.toolchain,
    TOOLCHAIN_KEYS,
    `${expected.targetGroup} toolchain`,
  );
  for (const key of TOOLCHAIN_KEYS) {
    requireNonEmptyString(
      metadata.toolchain?.[key],
      `${expected.targetGroup} toolchain.${key}`,
    );
  }
  assert(
    metadata.toolchain.node === "v24.19.0",
    `${expected.targetGroup} Node version drifted`,
  );
  assert(
    metadata.toolchain.pnpm === "10.12.3",
    `${expected.targetGroup} pnpm version drifted`,
  );
  assert(
    metadata.toolchain.rustc.startsWith("rustc 1.97.1 "),
    `${expected.targetGroup} Rust version drifted`,
  );

  return {
    schema: "fyagent-platform-build/v1",
    targetGroup: expected.targetGroup,
    platform: expected.platform,
    architecture: expected.architecture,
    runner: {
      requestedLabel: expected.requestedRunnerLabel,
      context: {
        os: metadata.runner.context.os,
        arch: metadata.runner.context.arch,
      },
    },
    container:
      expected.expectedContainer === null
        ? null
        : {
            configuredImage: {
              reference: metadata.container.configuredImage.reference,
              manifestDigest: metadata.container.configuredImage.manifestDigest,
            },
            observed: {
              osRelease: {
                id: metadata.container.observed.osRelease.id,
                versionId: metadata.container.observed.osRelease.versionId,
              },
              unameMachine: metadata.container.observed.unameMachine,
            },
          },
    toolchain: {
      node: metadata.toolchain.node,
      pnpm: metadata.toolchain.pnpm,
      rustc: metadata.toolchain.rustc,
    },
  };
}

export function buildBuildMetadata({
  metadataDirectory,
  identity,
  generatedAt,
}) {
  assertExactKeys(identity, IDENTITY_KEYS, "release identity");
  assertReleaseIdentity({
    version: identity.productVersion,
    tag: identity.tag,
    sourceSha: identity.sourceSha,
  });
  assert(
    identity.productVersion === FORMAL_VERSION,
    `Only FyAgent ${FORMAL_VERSION} is supported`,
  );
  assert(
    identity.repository === EXPECTED_REPOSITORY,
    "Repository identity drifted",
  );
  assert(
    String(identity.repositoryId) === EXPECTED_REPOSITORY_ID,
    "Repository ID drifted",
  );
  assert(
    identity.workflowPath === RELEASE_WORKFLOW_PATH,
    "Release workflow path drifted",
  );
  requireNonEmptyString(identity.workflowRef, "workflowRef");
  assert(
    identity.workflowRef.startsWith(
      `${EXPECTED_REPOSITORY}/${RELEASE_WORKFLOW_PATH}@`,
    ),
    "Release workflow ref drifted",
  );
  assert(
    ["push", "workflow_dispatch"].includes(identity.event),
    `Unsupported release event: ${identity.event}`,
  );
  assert(
    identity.mode === (identity.event === "push" ? "formal" : "preflight"),
    "Release mode does not match event",
  );
  assert(
    SHA_PATTERN.test(identity.workflowSha) &&
      identity.workflowSha === identity.sourceSha,
    "Trusted workflow SHA is invalid or differs from the attested source",
  );
  const workflowRefPrefix = `${EXPECTED_REPOSITORY}/${RELEASE_WORKFLOW_PATH}@`;
  if (identity.mode === "formal") {
    assert(
      identity.workflowRef === `${workflowRefPrefix}refs/tags/${FORMAL_TAG}`,
      "Formal Release workflow ref drifted",
    );
  } else {
    assert(
      identity.workflowRef === `${workflowRefPrefix}refs/heads/main`,
      "Preflight must use the trusted main workflow ref",
    );
  }
  assert(/^[1-9]\d*$/.test(String(identity.runId)), "runId must be numeric");
  assert(
    /^[1-9]\d*$/.test(String(identity.runAttempt)),
    "runAttempt must be numeric",
  );
  if (identity.mode === "formal") {
    assert(
      identity.ciWorkflowPath === CI_WORKFLOW_PATH,
      "CI workflow path drifted",
    );
    assert(
      /^[1-9]\d*$/.test(String(identity.ciRunId)),
      "ciRunId must be numeric",
    );
    assert(
      /^[1-9]\d*$/.test(String(identity.ciRunAttempt)),
      "ciRunAttempt must be numeric",
    );
  } else {
    assert(
      identity.ciWorkflowPath === null &&
        identity.ciRunId === null &&
        identity.ciRunAttempt === null,
      "Preflight metadata must not claim a Required CI binding",
    );
  }
  assert(
    typeof generatedAt === "string" &&
      new Date(generatedAt).toISOString() === generatedAt,
    "generatedAt must be an ISO-8601 instant",
  );

  const expectedFiles = EXPECTED_TARGETS.map(
    ({ targetGroup }) => `${targetGroup}.json`,
  );
  assertExactFileSet(
    metadataDirectory,
    expectedFiles,
    "platform metadata directory",
  );
  const targets = EXPECTED_TARGETS.map((expected) =>
    validatePlatformMetadata(
      readJson(join(metadataDirectory, `${expected.targetGroup}.json`)),
      expected,
      identity,
    ),
  );

  return {
    schema: "fyagent-build-metadata/v1",
    product: PRODUCT_NAME,
    version: identity.productVersion,
    tag: identity.tag,
    sourceSha: identity.sourceSha,
    repository: {
      nameWithOwner: identity.repository,
      id: String(identity.repositoryId),
    },
    workflow: {
      path: identity.workflowPath,
      ref: identity.workflowRef,
      sha: identity.workflowSha,
      runId: String(identity.runId),
      runAttempt: String(identity.runAttempt),
      event: identity.event,
      mode: identity.mode,
    },
    requiredCi:
      identity.mode === "formal"
        ? {
            path: identity.ciWorkflowPath,
            runId: String(identity.ciRunId),
            runAttempt: String(identity.ciRunAttempt),
            job: "CI / Required",
            conclusion: "success",
          }
        : null,
    generatedAt,
    targets,
  };
}
