#!/usr/bin/env node

import { writeFileSync } from "node:fs";
import {
  EXPECTED_TARGETS,
  GITHUB_RUNNER_ARCHITECTURES,
} from "./release-contract.mjs";

const [output] = process.argv.slice(2);
if (!output) {
  console.error(
    "Usage: node scripts/release/write-platform-metadata.mjs <output>",
  );
  process.exit(1);
}

function required(name) {
  const value = process.env[name];
  if (!value?.trim())
    throw new Error(`Required environment variable is missing: ${name}`);
  return value.trim();
}

function optional(name) {
  return process.env[name]?.trim() || null;
}

function requireExpected(name, actual, expected, targetGroup) {
  if (actual !== expected) {
    throw new Error(
      `${name} must be ${expected} for ${targetGroup}; received ${actual}`,
    );
  }
}

const CONTAINER_INPUT_NAMES = Object.freeze([
  "CONTAINER_IMAGE_REFERENCE",
  "CONTAINER_MANIFEST_DIGEST",
  "ACTUAL_CONTAINER_OS_ID",
  "ACTUAL_CONTAINER_OS_VERSION_ID",
  "ACTUAL_CONTAINER_UNAME_MACHINE",
]);

try {
  const targetGroup = required("TARGET_GROUP");
  const expected = EXPECTED_TARGETS.find(
    (candidate) => candidate.targetGroup === targetGroup,
  );
  if (!expected) {
    throw new Error(`Unsupported target group: ${targetGroup}`);
  }

  const platform = required("TARGET_PLATFORM");
  const architecture = required("TARGET_ARCHITECTURE");
  const requestedRunnerLabel = required("REQUESTED_RUNNER_LABEL");
  const runnerOs = required("ACTUAL_RUNNER_OS");
  const runnerArch = required("ACTUAL_RUNNER_ARCH");
  requireExpected("TARGET_PLATFORM", platform, expected.platform, targetGroup);
  requireExpected(
    "TARGET_ARCHITECTURE",
    architecture,
    expected.architecture,
    targetGroup,
  );
  requireExpected(
    "REQUESTED_RUNNER_LABEL",
    requestedRunnerLabel,
    expected.requestedRunnerLabel,
    targetGroup,
  );
  requireExpected(
    "ACTUAL_RUNNER_OS",
    runnerOs,
    expected.expectedRunnerOs,
    targetGroup,
  );
  if (!GITHUB_RUNNER_ARCHITECTURES.includes(runnerArch)) {
    throw new Error(
      `ACTUAL_RUNNER_ARCH is not a documented GitHub runner architecture: ${runnerArch}`,
    );
  }
  requireExpected(
    "ACTUAL_RUNNER_ARCH",
    runnerArch,
    expected.expectedRunnerArch,
    targetGroup,
  );

  let container;
  if (expected.expectedContainer === null) {
    const suppliedContainerInputs = CONTAINER_INPUT_NAMES.filter((name) =>
      process.env[name]?.trim(),
    );
    if (suppliedContainerInputs.length > 0) {
      throw new Error(
        `${targetGroup} must not supply container metadata inputs: ${suppliedContainerInputs.join(", ")}`,
      );
    }
    container = null;
  } else {
    const imageReference = required("CONTAINER_IMAGE_REFERENCE");
    const manifestDigest = required("CONTAINER_MANIFEST_DIGEST");
    const osReleaseId = required("ACTUAL_CONTAINER_OS_ID");
    const osReleaseVersionId = required("ACTUAL_CONTAINER_OS_VERSION_ID");
    const unameMachine = required("ACTUAL_CONTAINER_UNAME_MACHINE");
    if (!/^sha256:[0-9a-f]{64}$/.test(manifestDigest)) {
      throw new Error(
        "CONTAINER_MANIFEST_DIGEST must be a lowercase SHA-256 digest",
      );
    }
    if (!imageReference.endsWith(`@${manifestDigest}`)) {
      throw new Error(
        "CONTAINER_IMAGE_REFERENCE must end with CONTAINER_MANIFEST_DIGEST",
      );
    }
    requireExpected(
      "CONTAINER_IMAGE_REFERENCE",
      imageReference,
      expected.expectedContainer.imageReference,
      targetGroup,
    );
    requireExpected(
      "CONTAINER_MANIFEST_DIGEST",
      manifestDigest,
      expected.expectedContainer.manifestDigest,
      targetGroup,
    );
    requireExpected(
      "ACTUAL_CONTAINER_OS_ID",
      osReleaseId,
      expected.expectedContainer.osReleaseId,
      targetGroup,
    );
    requireExpected(
      "ACTUAL_CONTAINER_OS_VERSION_ID",
      osReleaseVersionId,
      expected.expectedContainer.osReleaseVersionId,
      targetGroup,
    );
    requireExpected(
      "ACTUAL_CONTAINER_UNAME_MACHINE",
      unameMachine,
      expected.expectedContainer.unameMachine,
      targetGroup,
    );
    container = {
      configuredImage: {
        reference: imageReference,
        manifestDigest,
      },
      observed: {
        osRelease: {
          id: osReleaseId,
          versionId: osReleaseVersionId,
        },
        unameMachine,
      },
    };
  }

  const mode = required("RELEASE_MODE");
  if (!(mode === "preflight" || mode === "formal")) {
    throw new Error(`Unsupported release mode: ${mode}`);
  }
  const ciRunId = optional("EXPECTED_CI_RUN_ID");
  const ciRunAttempt = optional("EXPECTED_CI_RUN_ATTEMPT");
  if (mode === "formal" && (!ciRunId || !ciRunAttempt)) {
    throw new Error(
      "Formal platform metadata requires the bound Required CI attempt",
    );
  }
  if (mode === "preflight" && (ciRunId || ciRunAttempt)) {
    throw new Error(
      "Preflight platform metadata must not claim a Required CI binding",
    );
  }
  const metadata = {
    schema: "fyagent-platform-build/v1",
    targetGroup,
    platform,
    architecture,
    runner: {
      requestedLabel: requestedRunnerLabel,
      context: {
        os: runnerOs,
        arch: runnerArch,
      },
    },
    container,
    toolchain: {
      node: required("ACTUAL_NODE_VERSION"),
      pnpm: required("ACTUAL_PNPM_VERSION"),
      rustc: required("ACTUAL_RUST_VERSION"),
    },
    identity: {
      productVersion: required("APP_VERSION"),
      tag: required("RELEASE_TAG"),
      sourceSha: required("SOURCE_SHA"),
      repository: required("GITHUB_REPOSITORY"),
      repositoryId: required("GITHUB_REPOSITORY_ID"),
      workflowPath: ".github/workflows/release.yml",
      workflowRef: required("GITHUB_WORKFLOW_REF"),
      workflowSha: required("GITHUB_WORKFLOW_SHA"),
      runId: required("GITHUB_RUN_ID"),
      runAttempt: required("GITHUB_RUN_ATTEMPT"),
      event: required("GITHUB_EVENT_NAME"),
      mode,
      ciWorkflowPath: mode === "formal" ? ".github/workflows/ci.yml" : null,
      ciRunId,
      ciRunAttempt,
    },
  };
  writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`, {
    flag: "wx",
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
