#!/usr/bin/env node

import { fail, run } from "./lib.mjs";

try {
  run("pnpm", ["run", "version:check"]);
  run("node", ["scripts/tasks/lockfile-check.mjs"]);
  run("node", ["scripts/tasks/task-contract-check.mjs"]);
  run("node", ["scripts/tasks/task-docs.mjs", "check"]);
  run("pnpm", [
    "exec",
    "vitest",
    "run",
    "tests/releaseWorkflow.test.ts",
    "tests/localBuildBoundary.test.ts",
    "tests/developmentEnvironment.test.ts",
    "tests/miseTaskContract.test.ts",
    "tests/taskDocs.test.ts",
    "tests/systemCheck.test.ts",
  ]);
} catch (error) {
  fail(error);
}
