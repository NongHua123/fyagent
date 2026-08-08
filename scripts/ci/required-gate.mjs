#!/usr/bin/env node

import process from "node:process";
import { pathToFileURL } from "node:url";

export const REQUIRED_CI_JOBS = Object.freeze([
  "contracts",
  "frontend",
  "desktop-acceptance-contract",
  "backend-linux",
  "backend-windows",
  "windows-msi-query",
  "backend-macos",
]);

const KNOWN_RESULTS = new Set(["success", "failure", "cancelled", "skipped"]);

function isPlainObject(value) {
  return (
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    (Object.getPrototypeOf(value) === Object.prototype ||
      Object.getPrototypeOf(value) === null)
  );
}

export function evaluateRequiredCiResults(value) {
  if (!isPlainObject(value)) {
    return {
      ok: false,
      requiredJobs: [...REQUIRED_CI_JOBS],
      results: {},
      errors: ["needs must be a JSON object keyed by required job id"],
    };
  }

  const results = {};
  const errors = [];
  const actualJobs = Object.keys(value).sort();
  const required = new Set(REQUIRED_CI_JOBS);

  for (const job of REQUIRED_CI_JOBS) {
    if (!Object.hasOwn(value, job)) {
      errors.push(`missing required job: ${job}`);
      continue;
    }
    const entry = value[job];
    if (!isPlainObject(entry) || typeof entry.result !== "string") {
      errors.push(`missing result for required job: ${job}`);
      continue;
    }
    results[job] = entry.result;
    if (!KNOWN_RESULTS.has(entry.result)) {
      errors.push(`unknown result for ${job}: ${entry.result}`);
    } else if (entry.result !== "success") {
      errors.push(`required job ${job} finished with ${entry.result}`);
    }
  }

  for (const job of actualJobs) {
    if (!required.has(job)) errors.push(`unexpected dependency job: ${job}`);
  }

  return {
    ok: errors.length === 0,
    requiredJobs: [...REQUIRED_CI_JOBS],
    results,
    errors,
  };
}

function parseCliResults(argv, env) {
  const argumentIndex = argv.indexOf("--results-json");
  const serialized =
    argumentIndex >= 0 ? argv[argumentIndex + 1] : env.REQUIRED_RESULTS;
  if (!serialized) {
    throw new Error(
      "Provide --results-json <json> or the REQUIRED_RESULTS environment variable",
    );
  }
  try {
    return JSON.parse(serialized);
  } catch (error) {
    throw new Error(
      `Required results are not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

export function runRequiredGateCli(
  argv = process.argv.slice(2),
  env = process.env,
) {
  let report;
  try {
    report = evaluateRequiredCiResults(parseCliResults(argv, env));
  } catch (error) {
    report = {
      ok: false,
      requiredJobs: [...REQUIRED_CI_JOBS],
      results: {},
      errors: [error instanceof Error ? error.message : String(error)],
    };
  }
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exitCode = 1;
  return report;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  runRequiredGateCli();
}
