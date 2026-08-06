import requirementsMatrix from "./requirements-matrix.json";

type RequirementEntry = (typeof requirementsMatrix.entries)[number];

export interface AcceptanceReportItem {
  id: string;
  requirements: readonly string[];
  status: "passed" | "covered" | "not-run";
  evidence: readonly string[];
}

export interface MockOnlyAcceptanceReport {
  mode: "mock-only";
  automated: AcceptanceReportItem[];
  coveredByExistingTests: AcceptanceReportItem[];
  notRunInThisEnvironment: AcceptanceReportItem[];
  acceptedRisks: readonly string[];
}

export function redactAcceptanceDiagnostic(value: string): string {
  return value
    .replace(
      /\b(api[_ -]?key|token|password)\s*[:=]\s*[^\s,;]+/gi,
      "$1=[REDACTED]",
    )
    .replace(/\bpid\s*[:=]\s*\d+/gi, "pid=[REDACTED]")
    .replace(/[A-Za-z]:\\Users\\[^\s,;]+/g, "[REDACTED_PATH]");
}

function toReportItem(
  entry: RequirementEntry,
  status: AcceptanceReportItem["status"],
): AcceptanceReportItem {
  return {
    id: entry.id,
    requirements: entry.requirements,
    status,
    evidence: entry.evidence,
  };
}

export function createMockOnlyAcceptanceReport(): MockOnlyAcceptanceReport {
  return {
    mode: "mock-only",
    automated: requirementsMatrix.entries
      .filter((entry) => entry.validation === "mock-contract")
      .map((entry) => toReportItem(entry, "passed")),
    coveredByExistingTests: requirementsMatrix.entries
      .filter(
        (entry) =>
          entry.validation === "existing-test" ||
          entry.validation === "supporting-test",
      )
      .map((entry) => toReportItem(entry, "covered")),
    notRunInThisEnvironment: requirementsMatrix.entries
      .filter((entry) => entry.validation === "candidate-only")
      .map((entry) => toReportItem(entry, "not-run")),
    acceptedRisks: [
      "No real desktop application, Codex/ChatGPT process, Windows installer, UAC prompt, or network endpoint was started.",
      "A mock contract is not evidence that a signed Windows candidate or WebDriver desktop run passed.",
    ],
  };
}
