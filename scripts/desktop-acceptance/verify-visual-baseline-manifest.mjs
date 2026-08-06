import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..", "..");
const manifestPath = path.join(
  repositoryRoot,
  "tests",
  "e2e",
  "visual-baselines",
  "manifest.json",
);
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const expectedLocales = ["en", "ja", "zh", "zh-TW"];
const expectedPlatforms = ["windows", "macos", "linux"];
const expectedScales = [100, 125, 150];

function requireCondition(condition, message) {
  if (!condition) {
    throw new Error(`Visual baseline preflight failed: ${message}`);
  }
}

requireCondition(
  manifest.captureMode === "candidate-only",
  "capture must be candidate-only",
);
requireCondition(
  manifest.stabilitySamples === 2,
  "two stable samples are required",
);
requireCondition(
  JSON.stringify(manifest.locales) === JSON.stringify(expectedLocales),
  "all four locales must be isolated",
);
requireCondition(
  JSON.stringify(manifest.platforms) === JSON.stringify(expectedPlatforms),
  "platform baselines must be isolated",
);
requireCondition(
  JSON.stringify(manifest.scales) === JSON.stringify(expectedScales),
  "DPI scale baselines must be isolated",
);
requireCondition(
  Array.isArray(manifest.regions) && manifest.regions.length > 0,
  "regions are required",
);

for (const region of manifest.regions) {
  requireCondition(
    region.pathTemplate.startsWith("{platform}/{scale}/{locale}/") &&
      region.pathTemplate.endsWith(".png"),
    `${region.id} must use the platform/scale/locale PNG layout`,
  );
}

console.log(
  JSON.stringify(
    {
      mode: "read-only-preflight",
      status: "ready-for-candidate-capture",
      regions: manifest.regions.map((region) => region.id),
      writes: false,
    },
    null,
    2,
  ),
);
