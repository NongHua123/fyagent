import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..", "..");
const manifest = JSON.parse(
  fs.readFileSync(
    path.join(
      repositoryRoot,
      "tests",
      "e2e",
      "visual-baselines",
      "manifest.json",
    ),
    "utf8",
  ),
);

function refuse(message) {
  console.error(`Refusing visual baseline update: ${message}`);
  process.exit(2);
}

if (process.env.FYAGENT_DESKTOP_ACCEPTANCE_MODE !== "candidate") {
  refuse(
    "set FYAGENT_DESKTOP_ACCEPTANCE_MODE=candidate in an approved candidate environment",
  );
}

if (process.env.FYAGENT_DESKTOP_BASELINE_REVIEW !== "approved") {
  refuse("set FYAGENT_DESKTOP_BASELINE_REVIEW=approved after human review");
}

const evidencePath = process.argv[2];
if (!evidencePath) {
  refuse(
    "supply a reviewed JSON evidence file; this command never writes PNG files",
  );
}

const evidence = JSON.parse(
  fs.readFileSync(path.resolve(evidencePath), "utf8"),
);
if (
  evidence.testBuild !== true ||
  evidence.fakeIpc !== true ||
  evidence.network !== "blocked" ||
  evidence.stableSamples < manifest.stabilitySamples ||
  !Array.isArray(evidence.artifacts) ||
  evidence.artifacts.length === 0
) {
  refuse(
    "evidence must show a test build, fake IPC, blocked network, stable samples, and artifacts",
  );
}

for (const artifact of evidence.artifacts) {
  const normalized = path.posix.normalize(artifact);
  if (
    !normalized.startsWith("tests/e2e/visual-baselines/") ||
    normalized.includes("..") ||
    !normalized.endsWith(".png")
  ) {
    refuse(
      "all candidate artifacts must be relative PNG paths below tests/e2e/visual-baselines",
    );
  }
}

console.log(
  JSON.stringify(
    {
      status: "approved-for-human-git-review",
      writes: false,
      artifacts: evidence.artifacts,
      nextStep:
        "Review the candidate PNGs and add them in a separate Git LFS-backed change.",
    },
    null,
    2,
  ),
);
