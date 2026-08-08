#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, fail } from "./lib.mjs";
import { generateTaskDocs } from "./task-docs.mjs";
import { loadTaskDefinitions } from "./task-contract-check.mjs";

const GENERATED_DOC = "docs/fyagent/development/mise-tasks.md";
const LEGACY_ENTRYPOINT_HANDOFF = new Set([
  ".github/pull_request_template.md",
  ".trellis/spec/backend/application-brand-assets.md",
  ".trellis/spec/backend/fyagent-v1-0-1-config-domains.md",
  ".trellis/spec/backend/fyagent-version-contract.md",
  ".trellis/spec/backend/github-release-workflow.md",
  ".trellis/spec/frontend/index.md",
  ".trellis/spec/frontend/quality-guidelines.md",
  ".trellis/spec/frontend/type-safety.md",
  "CONTRIBUTING.md",
  "README.md",
  "README_DE.md",
  "README_JA.md",
  "README_ZH.md",
]);

function walk(relativeRoot) {
  const absoluteRoot = path.join(ROOT, relativeRoot);
  if (!fs.existsSync(absoluteRoot)) return [];
  const files = [];
  const visit = (absolute) => {
    for (const entry of fs.readdirSync(absolute, { withFileTypes: true })) {
      const child = path.join(absolute, entry.name);
      if (entry.isDirectory()) visit(child);
      else if (entry.isFile() && entry.name.endsWith(".md")) {
        files.push(path.relative(ROOT, child).split(path.sep).join("/"));
      }
    }
  };
  if (fs.statSync(absoluteRoot).isFile()) return [relativeRoot];
  visit(absoluteRoot);
  return files;
}

try {
  const generatedPath = path.join(ROOT, GENERATED_DOC);
  if (!fs.existsSync(generatedPath))
    throw new Error(`Missing ${GENERATED_DOC}`);
  const committed = fs
    .readFileSync(generatedPath, "utf8")
    .replace(/\r\n/g, "\n");
  if (committed !== generateTaskDocs()) {
    throw new Error("Generated task documentation is stale");
  }

  const tasks = loadTaskDefinitions();
  for (const match of committed.matchAll(/mise run ([a-z0-9:.-]+)/gi)) {
    const name = match[1];
    if (name === "<task>" || name === "tasks:docs:generate") continue;
    if (!tasks[name])
      throw new Error(`Task docs reference unknown task: ${name}`);
  }

  const activeDocs = [
    ...[
      "README.md",
      "README_ZH.md",
      "README_JA.md",
      "README_DE.md",
      "CONTRIBUTING.md",
    ],
    ...walk(".github").filter((file) => file.endsWith(".md")),
    ...walk(".trellis/spec/backend"),
    ...walk(".trellis/spec/frontend"),
  ];
  const legacy = [];
  for (const file of [...new Set(activeDocs)].sort()) {
    const source = fs.readFileSync(path.join(ROOT, file), "utf8");
    if (!source.includes("mise exec --")) continue;
    if (!LEGACY_ENTRYPOINT_HANDOFF.has(file)) {
      throw new Error(
        `Untracked legacy mise exec entrypoint in active document: ${file}`,
      );
    }
    legacy.push(file);
  }
  console.log(
    JSON.stringify(
      {
        ok: true,
        generated: GENERATED_DOC,
        legacyEntrypointHandoff: legacy,
        handoffOwner: "08-07-migrate-docs-and-trellis-specs",
      },
      null,
      2,
    ),
  );
} catch (error) {
  fail(error);
}
