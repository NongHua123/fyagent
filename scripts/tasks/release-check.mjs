#!/usr/bin/env node

import { fail, run } from "./lib.mjs";

const CI_SAFE_TESTS = Object.freeze([
  "tests/releaseWorkflow.test.ts",
  "tests/downloadManifest.test.ts",
  "tests/releaseAssets.test.ts",
  "tests/githubWorkflowTriggers.test.ts",
  "tests/ciWorkflow.test.ts",
  "tests/requiredCiGate.test.ts",
  "tests/ciToolchainContract.test.ts",
  "tests/dep0040Contract.test.ts",
  "tests/localBuildBoundary.test.ts",
]);

const LOCAL_MISE_TESTS = Object.freeze([
  "tests/developmentEnvironment.test.ts",
  "tests/developmentHooks.test.ts",
  "tests/miseTaskContract.test.ts",
  "tests/taskDocs.test.ts",
  "tests/systemCheck.test.ts",
]);

try {
  const args = process.argv.slice(2);
  const ciMode = args.length === 1 && args[0] === "--ci";
  if (args.length > 0 && !ciMode) {
    throw new Error("Usage: release-check.mjs [--ci]");
  }

  run("pnpm", ["run", "version:check"]);
  run("node", ["scripts/tasks/lockfile-check.mjs"]);
  run("node", ["scripts/tasks/dep0040-check.mjs"]);
  if (!ciMode) {
    run("node", ["scripts/tasks/task-contract-check.mjs"]);
  }
  run("node", ["scripts/tasks/task-docs.mjs", "check"]);
  run("pnpm", [
    "run",
    "test:unit",
    ...CI_SAFE_TESTS,
    ...(ciMode ? [] : LOCAL_MISE_TESTS),
  ]);
  if (!ciMode) {
    run("pnpm", ["run", "test:native-fetch"]);
  }
} catch (error) {
  fail(error);
}
