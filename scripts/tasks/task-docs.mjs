#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { ROOT, fail, isMain, usageBoolean } from "./lib.mjs";
import { loadTaskDefinitions } from "./task-contract-check.mjs";

const OUTPUT = path.join(
  ROOT,
  "docs",
  "fyagent",
  "development",
  "mise-tasks.md",
);

const SECTIONS = Object.freeze([
  [
    "Setup and Checks",
    (name) =>
      [
        "bootstrap",
        "deps:install",
        "env:check",
        "system:check",
        "check",
        "check:frontend",
        "check:backend",
        "check:contracts",
      ].includes(name),
  ],
  [
    "Development and Native Build",
    (name) =>
      name === "dev" ||
      name.startsWith("dev:") ||
      name === "build" ||
      name.startsWith("build:"),
  ],
  [
    "Frontend and Desktop Tests",
    (name) =>
      ["typecheck", "format", "format:check", "test"].includes(name) ||
      name.startsWith("test:"),
  ],
  ["Rust", (name) => name.startsWith("rust:")],
  ["Python and uv", (name) => name.startsWith("python:")],
  [
    "Trellis and Codex Hooks",
    (name) => name.startsWith("trellis:") || name.startsWith("codex:"),
  ],
  [
    "Version, Assets, and Cleanup",
    (name) =>
      name.startsWith("version:") ||
      name.startsWith("assets:") ||
      name.startsWith("clean:"),
  ],
  [
    "Dependency and Toolchain Maintenance",
    (name) => name.startsWith("deps:") || name.startsWith("toolchain:"),
  ],
  ["Upstream", (name) => name.startsWith("upstream:")],
  ["Task Metadata and Documentation", (name) => name.startsWith("tasks:")],
  ["Release Contract", (name) => name.startsWith("release:")],
]);

export function escapeMarkdownCell(value) {
  return String(value ?? "")
    .replace(/\r?\n/g, " ")
    .replace(/\|/g, "\\|")
    .replace(/\s+/g, " ")
    .trim();
}

function usageSummary(usage) {
  if (!usage) return "—";
  const parts = [...usage.matchAll(/(?:arg|flag)\s+"([^"]+)"/g)].map(
    (match) => match[1],
  );
  return parts.length > 0 ? parts.join(" ") : "see `mise run <task> --help`";
}

function renderMarkdownTable(headers, rows) {
  const widths = headers.map((header, index) =>
    Math.max(3, header.length, ...rows.map((row) => row[index].length)),
  );
  const renderRow = (row) =>
    `| ${row.map((cell, index) => cell.padEnd(widths[index])).join(" | ")} |`;

  return [
    renderRow(headers),
    renderRow(widths.map((width) => "-".repeat(width))),
    ...rows.map(renderRow),
  ];
}

export function generateTaskDocs() {
  const tasks = loadTaskDefinitions();
  const assigned = new Set();
  const lines = [
    "# FyAgent mise Task Reference",
    "",
    "> Generated from `.mise/tasks/*.toml` by `mise run tasks:docs:generate --apply`.",
    "> Do not edit task rows by hand; `mise run tasks:docs:check` performs a byte comparison.",
    "",
    "Use `mise run <task>`. GitHub Actions is the explicit non-mise execution boundary.",
    "Parameterized tasks expose their contract through `mise run <task> --help`.",
    "Tasks marked preview-by-default require `--apply` before they write or delete repository state.",
    "",
    "Standard versions: Node 24.19.0, pnpm 10.12.3, Rust 1.97.1, Python 3.14.7;",
    "the approved `uv = latest` resolution is pinned in `mise.lock`.",
    "",
  ];
  const renderSection = (title, names) => {
    const rows = names.sort().map((name) => {
      const task = tasks[name];
      assigned.add(name);
      return [
        `\`${escapeMarkdownCell(name)}\``,
        escapeMarkdownCell(task.description),
        escapeMarkdownCell(usageSummary(task.usage)),
        escapeMarkdownCell(task.env?.FYAGENT_TASK_EFFECT),
      ];
    });
    lines.push(
      `## ${title}`,
      "",
      ...renderMarkdownTable(["Task", "Description", "Usage", "Effect"], rows),
    );
    lines.push("");
  };
  for (const [title, predicate] of SECTIONS) {
    const names = Object.keys(tasks).filter(
      (name) => !assigned.has(name) && predicate(name),
    );
    if (names.length > 0) renderSection(title, names);
  }
  const extra = Object.keys(tasks).filter((name) => !assigned.has(name));
  if (extra.length > 0) renderSection("Additional Tasks", extra);
  lines.push(
    "## Safety Boundaries",
    "",
    "- `bootstrap` never changes trust, system packages, Git remotes, locks, tags, or releases.",
    "- `check` reaches read-only tasks only; Rust checks remain ordered fmt → check → Clippy → test.",
    "- `dev` and `build*` are current-host only and expose no cross-OS or cross-architecture target.",
    "- Clean tasks canonicalize an allowlisted repository child path and never remove Git, Trellis, locks, historical baselines, or user data.",
    "- Upstream tasks do not change remotes, resolve conflicts, commit, tag, or push.",
    "- `release:check` is read-only; only GitHub Actions may publish a formal release.",
    "",
  );
  return `${lines.join("\n").replace(/\n+$/, "")}\n`;
}

function main() {
  const command = process.argv[2];
  const generated = generateTaskDocs();
  if (command === "check") {
    if (
      !fs.existsSync(OUTPUT) ||
      fs.readFileSync(OUTPUT, "utf8").replace(/\r\n/g, "\n") !== generated
    ) {
      throw new Error(
        "Task documentation is stale; run mise run tasks:docs:generate --apply",
      );
    }
    console.log("Task documentation is byte-for-byte current.");
    return;
  }
  if (command !== "generate")
    throw new Error(`Unknown task docs command: ${command ?? ""}`);
  if (!usageBoolean("apply")) {
    process.stdout.write(generated);
    return;
  }
  fs.mkdirSync(path.dirname(OUTPUT), { recursive: true });
  fs.writeFileSync(OUTPUT, generated);
  console.log(`Updated ${path.relative(ROOT, OUTPUT)}.`);
}

if (isMain(import.meta.url)) {
  try {
    main();
  } catch (error) {
    fail(error);
  }
}
