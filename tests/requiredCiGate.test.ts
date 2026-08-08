import { spawnSync } from "node:child_process";
import path from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error The workflow executes this dependency-free JavaScript helper directly.
import * as requiredGateModule from "../scripts/ci/required-gate.mjs";

type GateReport = {
  ok: boolean;
  requiredJobs: string[];
  results: Record<string, string>;
  errors: string[];
};

const ROOT = path.resolve(__dirname, "..");
const REQUIRED_JOB_IDS =
  requiredGateModule.REQUIRED_CI_JOBS as readonly string[];
const EXPECTED_REQUIRED_JOB_IDS = [
  "contracts",
  "frontend",
  "desktop-acceptance-contract",
  "backend-linux",
  "backend-windows",
  "windows-msi-query",
  "backend-macos",
] as const;
const evaluateRequiredCiResults =
  requiredGateModule.evaluateRequiredCiResults as (
    value: unknown,
  ) => GateReport;

function needs(result = "success"): Record<string, { result: string }> {
  return Object.fromEntries(
    REQUIRED_JOB_IDS.map((job: string) => [job, { result }]),
  );
}

describe("CI / Required gate", () => {
  it("accepts only the exact seven successful dependency jobs", () => {
    expect(REQUIRED_JOB_IDS).toEqual(EXPECTED_REQUIRED_JOB_IDS);
    const report = evaluateRequiredCiResults(needs());
    expect(report).toEqual({
      ok: true,
      requiredJobs: [...REQUIRED_JOB_IDS],
      results: Object.fromEntries(
        REQUIRED_JOB_IDS.map((job: string) => [job, "success"]),
      ),
      errors: [],
    });
  });

  it.each(["failure", "cancelled", "skipped"])(
    "rejects a %s dependency result",
    (result) => {
      const input = needs();
      input.frontend.result = result;
      const report = evaluateRequiredCiResults(input);
      expect(report.ok).toBe(false);
      expect(report.errors).toContain(
        `required job frontend finished with ${result}`,
      );
    },
  );

  it("rejects missing, extra, missing-result, and unknown-result entries", () => {
    const missing = needs();
    delete missing["backend-macos"];
    expect(evaluateRequiredCiResults(missing).errors).toContain(
      "missing required job: backend-macos",
    );

    const extra = { ...needs(), optional: { result: "success" } };
    expect(evaluateRequiredCiResults(extra).errors).toContain(
      "unexpected dependency job: optional",
    );

    const missingResult = needs() as Record<string, { result?: string }>;
    delete missingResult.contracts.result;
    expect(evaluateRequiredCiResults(missingResult).errors).toContain(
      "missing result for required job: contracts",
    );

    const unknown = needs();
    unknown["backend-linux"].result = "neutral";
    expect(evaluateRequiredCiResults(unknown).errors).toContain(
      "unknown result for backend-linux: neutral",
    );
  });

  it("fails closed on malformed CLI input and emits a machine-readable report", () => {
    const result = spawnSync(
      process.execPath,
      ["scripts/ci/required-gate.mjs", "--results-json", "{"],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(result.status).toBe(1);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      errors: string[];
    };
    expect(report.ok).toBe(false);
    expect(report.errors[0]).toContain("not valid JSON");
  });
});
