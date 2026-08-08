#!/usr/bin/env node

import { writeFileSync } from "node:fs";

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

try {
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
    targetGroup: required("TARGET_GROUP"),
    platform: required("TARGET_PLATFORM"),
    architecture: required("TARGET_ARCHITECTURE"),
    runnerLabel: required("RUNNER_LABEL"),
    runner: {
      runnerOs: required("RUNNER_OS"),
      runnerArch: required("RUNNER_ARCH"),
      imageOs: required("ImageOS"),
      imageVersion: required("ImageVersion"),
    },
    containerDigest: process.env.CONTAINER_DIGEST?.trim() || null,
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
