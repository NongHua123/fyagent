import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..", "..");

function readJson(relativePath) {
  return JSON.parse(
    fs.readFileSync(path.join(repositoryRoot, relativePath), "utf8"),
  );
}

function requireCondition(condition, message) {
  if (!condition) {
    throw new Error(`Desktop acceptance contract failed: ${message}`);
  }
}

const packageJson = readJson("package.json");
const matrix = readJson("tests/desktop-acceptance/requirements-matrix.json");
const manifest = readJson("tests/e2e/visual-baselines/manifest.json");
const workflow = fs.readFileSync(
  path.join(repositoryRoot, ".github", "workflows", "ci.yml"),
  "utf8",
);
const gitAttributes = fs.readFileSync(
  path.join(repositoryRoot, ".gitattributes"),
  "utf8",
);

requireCondition(
  packageJson.scripts["test:desktop:mock"] ===
    "vitest run tests/desktop-acceptance && node scripts/desktop-acceptance/verify-mock-contract.mjs",
  "test:desktop:mock must run the isolated Vitest contract before this verifier",
);
requireCondition(
  packageJson.scripts["test:desktop:visual:preflight"] ===
    "node scripts/desktop-acceptance/verify-visual-baseline-manifest.mjs",
  "visual preflight must be a read-only manifest verifier",
);
requireCondition(
  workflow.includes("desktop-acceptance-contract:"),
  "manual CI must include the desktop acceptance contract job",
);
requireCondition(
  workflow.includes("run: pnpm test:desktop:mock"),
  "manual CI must run the mock-only acceptance contract",
);
requireCondition(
  workflow.includes("run: pnpm test:desktop:visual:preflight"),
  "manual CI must validate the visual-baseline policy",
);
requireCondition(
  gitAttributes.includes(
    "tests/e2e/visual-baselines/**/*.png filter=lfs diff=lfs merge=lfs -text",
  ),
  "visual baseline PNGs must remain in Git LFS",
);
requireCondition(
  manifest.captureMode === "candidate-only" && manifest.stabilitySamples === 2,
  "visual baselines must require candidate-only capture and two stable samples",
);
requireCondition(
  Array.isArray(matrix.entries) && matrix.entries.length > 0,
  "the requirements matrix must not be empty",
);

for (const entry of matrix.entries) {
  requireCondition(
    Array.isArray(entry.requirements) && entry.requirements.length > 0,
    `${entry.id} must list at least one requirement`,
  );
  requireCondition(
    Array.isArray(entry.evidence) && entry.evidence.length > 0,
    `${entry.id} must list tracked evidence`,
  );
}

console.log(
  JSON.stringify(
    {
      mode: "mock-only",
      automated: [
        {
          id: "QA-MOCK-001",
          status: "passed",
          evidence:
            "tests/desktop-acceptance/desktopAcceptanceContract.test.ts",
        },
      ],
      coveredByExistingTests: matrix.entries
        .filter(
          (entry) =>
            entry.validation === "existing-test" ||
            entry.validation === "supporting-test",
        )
        .map((entry) => entry.id),
      notRunInThisEnvironment: matrix.entries
        .filter((entry) => entry.validation === "candidate-only")
        .map((entry) => entry.id),
      acceptedRisks: [
        "No real desktop application or external process was started.",
        "No Windows installer, UAC, signing, or release candidate was exercised.",
      ],
    },
    null,
    2,
  ),
);
