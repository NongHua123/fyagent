#!/usr/bin/env node
// Generates the release download manifest from assets that have already passed
// the release workflow's platform gates. Version, tag, and source SHA are
// explicit inputs from the frozen version-contract job; this script never
// derives a version by trimming the tag.
//
// Usage:
//   node scripts/generate-download-manifest.mjs \
//     <assets-dir> <app-version> <release-tag> <source-sha> <base-url> [output]

import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [assetsDir, appVersion, releaseTag, sourceSha, baseUrl, output = "manifest.json"] =
  process.argv.slice(2);

if (!assetsDir || !appVersion || !releaseTag || !sourceSha || !baseUrl) {
  console.error(
    "Usage: node scripts/generate-download-manifest.mjs <assets-dir> <app-version> <release-tag> <source-sha> <base-url> [output]",
  );
  process.exit(1);
}

if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(appVersion)) {
  console.error(`Invalid application version: ${appVersion}`);
  process.exit(1);
}
if (releaseTag !== `v${appVersion}`) {
  console.error(
    `Release tag must exactly match v${appVersion}; received ${releaseTag}`,
  );
  process.exit(1);
}
if (!/^[0-9a-f]{40}$/i.test(sourceSha)) {
  console.error("source-sha must be a full 40-character Git commit SHA");
  process.exit(1);
}

// Longer suffixes must come before their shorter counterparts
// (e.g. -Windows-arm64.msi before -Windows.msi).
const RULES = [
  { suffix: "-macOS.dmg", platform: "macos", kind: "dmg", arch: "universal" },
  { suffix: "-macOS.zip", platform: "macos", kind: "zip", arch: "universal" },
  {
    suffix: "-Windows-arm64.msi",
    platform: "windows",
    kind: "msi",
    arch: "arm64",
  },
  { suffix: "-Windows.msi", platform: "windows", kind: "msi", arch: "x64" },
  {
    suffix: "-Linux-arm64.AppImage",
    platform: "linux",
    kind: "appimage",
    arch: "arm64",
  },
  {
    suffix: "-Linux-x86_64.AppImage",
    platform: "linux",
    kind: "appimage",
    arch: "x64",
  },
  { suffix: "-Linux-arm64.deb", platform: "linux", kind: "deb", arch: "arm64" },
  { suffix: "-Linux-x86_64.deb", platform: "linux", kind: "deb", arch: "x64" },
  { suffix: "-Linux-arm64.rpm", platform: "linux", kind: "rpm", arch: "arm64" },
  { suffix: "-Linux-x86_64.rpm", platform: "linux", kind: "rpm", arch: "x64" },
];

const normalizedBase = baseUrl.replace(/\/+$/, "");
const assets = [];
const expectedPrefix = `FyAgent-${appVersion}-`;

for (const name of readdirSync(assetsDir).sort()) {
  // Unmatched files (.sig, updater artifacts, and unrelated workflow output)
  // are deliberately excluded from the user-facing download manifest.
  const rule = RULES.find((entry) => name.endsWith(entry.suffix));
  if (!rule) continue;
  if (!name.startsWith(expectedPrefix)) {
    console.error(
      `Release asset does not use the frozen application version ${appVersion}: ${name}`,
    );
    process.exit(1);
  }

  const assetPath = join(assetsDir, name);
  assets.push({
    platform: rule.platform,
    kind: rule.kind,
    arch: rule.arch,
    name,
    size: statSync(assetPath).size,
    sha256: createHash("sha256").update(readFileSync(assetPath)).digest("hex"),
    url: `${normalizedBase}/${releaseTag}/${encodeURIComponent(name)}`,
  });
}

if (assets.length === 0) {
  console.error(`No release assets matched in ${assetsDir}`);
  process.exit(1);
}

const manifest = {
  schema: "fyagent-download-manifest/v1",
  version: appVersion,
  tag: releaseTag,
  sourceSha,
  pubDate: new Date().toISOString(),
  assets,
};

writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(
  `Wrote ${output} with ${assets.length} assets for ${releaseTag} at ${sourceSha}`,
);
