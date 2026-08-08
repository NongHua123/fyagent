#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { ROOT, fail, repositoryPath, usageBoolean } from "./lib.mjs";

const TARGETS = Object.freeze({
  frontend: ["node_modules", "dist"],
  rust: ["src-tauri/target"],
  python: [".venv"],
  artifacts: ["release", "artifacts", "flatpak-build", "flatpak-repo"],
});

function selectedTargets(domain) {
  if (domain === "all") {
    return [...new Set(Object.values(TARGETS).flat())];
  }
  const targets = TARGETS[domain];
  if (!targets) throw new Error(`Unknown clean domain: ${domain}`);
  return targets;
}

try {
  const domain = process.argv[2];
  const targets = selectedTargets(domain).map((relative) => ({
    relative,
    absolute: repositoryPath(relative),
  }));
  const protectedRoots = new Set([
    path.join(ROOT, ".git"),
    path.join(ROOT, ".trellis"),
  ]);
  for (const target of targets) {
    if (protectedRoots.has(target.absolute)) {
      throw new Error(`Refusing protected clean target: ${target.relative}`);
    }
  }

  const existing = targets.filter(({ absolute }) => fs.existsSync(absolute));
  const apply = usageBoolean("apply");
  console.log(
    JSON.stringify(
      {
        status: apply ? "applying" : "preview",
        domain,
        targets: existing.map(({ relative }) => relative),
        absent: targets
          .filter(({ absolute }) => !fs.existsSync(absolute))
          .map(({ relative }) => relative),
      },
      null,
      2,
    ),
  );
  if (apply) {
    for (const { absolute } of existing) {
      fs.rmSync(absolute, { recursive: true, force: false, maxRetries: 2 });
    }
  }
} catch (error) {
  fail(error);
}
