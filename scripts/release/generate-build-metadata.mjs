#!/usr/bin/env node

import { writeFileSync } from "node:fs";
import { buildBuildMetadata } from "./release-contract.mjs";

const [
  metadataDirectory,
  version,
  tag,
  sourceSha,
  repository,
  repositoryId,
  runId,
  runAttempt,
  event,
  mode,
  workflowRef,
  workflowSha,
  ciRunId,
  ciRunAttempt,
  generatedAt,
  output = "build-metadata.json",
] = process.argv.slice(2);

if (
  !metadataDirectory ||
  !version ||
  !tag ||
  !sourceSha ||
  !repository ||
  !repositoryId ||
  !runId ||
  !runAttempt ||
  !event ||
  !mode ||
  !workflowRef ||
  !workflowSha ||
  !generatedAt
) {
  console.error(
    "Usage: node scripts/release/generate-build-metadata.mjs <metadata-dir> <version> <tag> <source-sha> <repository> <repository-id> <run-id> <run-attempt> <event> <mode> <workflow-ref> <workflow-sha> <ci-run-id-or-empty> <ci-run-attempt-or-empty> <generated-at> [output]",
  );
  process.exit(1);
}

try {
  const metadata = buildBuildMetadata({
    metadataDirectory,
    identity: {
      productVersion: version,
      tag,
      sourceSha,
      repository,
      repositoryId,
      workflowPath: ".github/workflows/release.yml",
      workflowRef,
      workflowSha,
      runId,
      runAttempt,
      event,
      mode,
      ciWorkflowPath: mode === "formal" ? ".github/workflows/ci.yml" : null,
      ciRunId: ciRunId || null,
      ciRunAttempt: ciRunAttempt || null,
    },
    generatedAt,
  });
  writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`, {
    flag: "wx",
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
