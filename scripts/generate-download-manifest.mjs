#!/usr/bin/env node

import { writeFileSync } from "node:fs";
import { buildDownloadManifest } from "./release/release-contract.mjs";

const [
  assetsDirectory,
  version,
  tag,
  sourceSha,
  baseUrl,
  publishedAt,
  output = "download-manifest.json",
] = process.argv.slice(2);

if (
  !assetsDirectory ||
  !version ||
  !tag ||
  !sourceSha ||
  !baseUrl ||
  !publishedAt
) {
  console.error(
    "Usage: node scripts/generate-download-manifest.mjs <assets-dir> <version> <tag> <source-sha> <base-url> <published-at> [output]",
  );
  process.exit(1);
}

try {
  const manifest = await buildDownloadManifest({
    assetsDirectory,
    version,
    tag,
    sourceSha,
    baseUrl,
    publishedAt,
  });
  writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`, {
    flag: "wx",
  });
  console.log(
    `Wrote ${output} with exactly ${manifest.assets.length} installers for ${tag}`,
  );
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
