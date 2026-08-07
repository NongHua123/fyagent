#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { ROOT, capture, fail, isMain, read, readToml } from "./lib.mjs";

export const REQUIRED_TASKS = Object.freeze([
  "assets:icons",
  "assets:icons:check",
  "bootstrap",
  "build",
  "build:binary",
  "build:debug",
  "build:renderer",
  "check",
  "check:backend",
  "check:contracts",
  "check:frontend",
  "clean:all",
  "clean:artifacts",
  "clean:frontend",
  "clean:python",
  "clean:rust",
  "codex:hook:subagent-context",
  "codex:hook:workflow-state",
  "codex:hooks:check",
  "deps:install",
  "deps:outdated",
  "deps:outdated:frontend",
  "deps:outdated:python",
  "deps:outdated:rust",
  "deps:update:frontend",
  "deps:update:rust",
  "dev",
  "dev:renderer",
  "env:check",
  "format",
  "format:check",
  "python:add:dev",
  "python:check",
  "python:lock",
  "python:lock:check",
  "python:remove:dev",
  "python:run",
  "python:sync",
  "python:tool",
  "python:update",
  "python:with",
  "release:check",
  "rust:check",
  "rust:clippy",
  "rust:fmt",
  "rust:fmt:check",
  "rust:test",
  "system:check",
  "tasks:docs:check",
  "tasks:docs:generate",
  "tasks:validate",
  "test",
  "test:desktop:mock",
  "test:desktop:visual:preflight",
  "test:desktop:visual:update",
  "test:i18n",
  "test:unit",
  "test:unit:watch",
  "toolchain:lock",
  "toolchain:outdated",
  "toolchain:update:node",
  "toolchain:update:pnpm",
  "toolchain:update:rust",
  "toolchain:update:uv",
  "trellis:context",
  "trellis:get-developer",
  "trellis:init-developer",
  "trellis:session:add",
  "trellis:task",
  "trellis:validate",
  "typecheck",
  "upstream:audit",
  "upstream:check",
  "upstream:fetch",
  "upstream:merge:abort",
  "upstream:merge:prepare",
  "version:bump",
  "version:check",
  "version:get",
  "version:set",
]);

export const PARAMETERIZED_TASKS = Object.freeze([
  "assets:icons",
  "clean:all",
  "clean:artifacts",
  "clean:frontend",
  "clean:python",
  "clean:rust",
  "deps:update:frontend",
  "deps:update:rust",
  "env:check",
  "python:add:dev",
  "python:lock",
  "python:remove:dev",
  "python:run",
  "python:tool",
  "python:update",
  "python:with",
  "rust:test",
  "system:check",
  "tasks:docs:generate",
  "test:desktop:visual:update",
  "test:unit",
  "test:unit:watch",
  "toolchain:lock",
  "toolchain:update:node",
  "toolchain:update:pnpm",
  "toolchain:update:rust",
  "toolchain:update:uv",
  "trellis:context",
  "trellis:init-developer",
  "trellis:session:add",
  "trellis:task",
  "trellis:validate",
  "upstream:audit",
  "upstream:fetch",
  "upstream:merge:prepare",
  "version:bump",
  "version:check",
  "version:set",
]);

export const RAW_TASKS = Object.freeze([
  "codex:hook:subagent-context",
  "codex:hook:workflow-state",
  "codex:hooks:check",
]);

const RETIRED_TASKS = Object.freeze([
  "macos:preflight",
  "build:cross-windows:x64",
  "build:cross-windows:arm64",
  "build:cross-windows",
  "build:cross-macos:universal",
]);

const EFFECTS = new Set([
  "build-output",
  "dependency-environment",
  "ephemeral-environment",
  "git-fetch",
  "git-state",
  "interactive",
  "preview-by-default",
  "read-only",
  "source-modifying",
  "trellis-state",
  "user-command",
]);

export function loadTaskDefinitions() {
  const directory = path.join(ROOT, ".mise", "tasks");
  const files = fs
    .readdirSync(directory)
    .filter((name) => name.endsWith(".toml"))
    .sort();
  const tasks = {};
  for (const file of files) {
    const parsed = readToml(path.posix.join(".mise/tasks", file));
    for (const [name, task] of Object.entries(parsed)) {
      if (tasks[name]) throw new Error(`Duplicate task definition: ${name}`);
      tasks[name] = { ...task, sourceFile: `.mise/tasks/${file}` };
    }
  }
  return tasks;
}

function taskReferences(task) {
  const references = [];
  const visit = (entry) => {
    if (!entry || typeof entry !== "object") return;
    if (typeof entry.task === "string") references.push(entry.task);
    if (Array.isArray(entry.tasks)) references.push(...entry.tasks);
  };
  const depends = Array.isArray(task.depends)
    ? task.depends
    : task.depends
      ? [task.depends]
      : [];
  for (const entry of depends) {
    if (typeof entry === "string") references.push(entry.split(/\s+/)[0]);
    else visit(entry);
  }
  const run = Array.isArray(task.run) ? task.run : [];
  for (const entry of run) visit(entry);
  return references;
}

function sequence(task) {
  return (Array.isArray(task.run) ? task.run : [])
    .filter((entry) => entry && typeof entry === "object" && entry.task)
    .map((entry) => entry.task);
}

function executableFiles() {
  const roots = ["mise.toml", ".mise/tasks", "scripts/tasks", ".codex"];
  const result = [];
  const visit = (absolute) => {
    const stat = fs.statSync(absolute);
    if (stat.isFile()) {
      if (!absolute.includes(`${path.sep}__pycache__${path.sep}`))
        result.push(absolute);
      return;
    }
    for (const entry of fs.readdirSync(absolute, { withFileTypes: true })) {
      visit(path.join(absolute, entry.name));
    }
  };
  for (const relative of roots) {
    const absolute = path.join(ROOT, relative);
    if (fs.existsSync(absolute)) visit(absolute);
  }
  return result;
}

export function validateTaskContract() {
  const tasks = loadTaskDefinitions();
  const names = Object.keys(tasks);
  for (const name of REQUIRED_TASKS) {
    if (!tasks[name]) throw new Error(`Missing canonical task: ${name}`);
  }
  for (const name of RETIRED_TASKS) {
    if (tasks[name]) throw new Error(`Retired task was reintroduced: ${name}`);
  }

  for (const [name, task] of Object.entries(tasks)) {
    if (
      typeof task.description !== "string" ||
      task.description.trim() === ""
    ) {
      throw new Error(`Task ${name} has no description`);
    }
    const effect = task.env?.FYAGENT_TASK_EFFECT;
    if (!EFFECTS.has(effect)) {
      throw new Error(
        `Task ${name} has invalid or missing FYAGENT_TASK_EFFECT`,
      );
    }
    for (const reference of taskReferences(task)) {
      if (!tasks[reference])
        throw new Error(`Task ${name} references missing task ${reference}`);
    }
  }

  const tasksWithUsage = names
    .filter(
      (name) =>
        typeof tasks[name].usage === "string" &&
        tasks[name].usage.trim() !== "",
    )
    .sort();
  if (
    JSON.stringify(tasksWithUsage) !==
    JSON.stringify([...PARAMETERIZED_TASKS].sort())
  ) {
    throw new Error(
      "Parameterized task set differs from the canonical usage-contract list",
    );
  }
  for (const name of PARAMETERIZED_TASKS) {
    if (
      typeof tasks[name].usage !== "string" ||
      tasks[name].usage.trim() === ""
    ) {
      throw new Error(
        `Parameterized task ${name} must declare non-empty usage`,
      );
    }
  }
  for (const [name, task] of Object.entries(tasks)) {
    const effect = task.env.FYAGENT_TASK_EFFECT;
    if (
      effect === "preview-by-default" &&
      !task.usage?.includes('flag "--apply"')
    ) {
      throw new Error(
        `${name} preview task must declare an explicit --apply flag`,
      );
    }
    if ((effect === "interactive") !== (task.interactive === true)) {
      throw new Error(`${name} interactive metadata and effect must agree`);
    }
  }
  const rawTasks = names.filter((name) => tasks[name].raw === true).sort();
  if (JSON.stringify(rawTasks) !== JSON.stringify([...RAW_TASKS].sort())) {
    throw new Error("Only the canonical Codex hook tasks may use raw mode");
  }
  const mergeAbort = tasks["upstream:merge:abort"];
  if (
    mergeAbort.env.FYAGENT_TASK_EFFECT !== "git-state" ||
    mergeAbort.confirm?.default !== "no" ||
    typeof mergeAbort.confirm?.message !== "string"
  ) {
    throw new Error(
      "upstream:merge:abort must be git-state with default-no confirmation",
    );
  }

  const loaded = JSON.parse(
    capture("mise", ["tasks", "ls", "--local", "--json"]),
  );
  const loadedNames = loaded.map((task) => task.name).sort();
  if (JSON.stringify(loadedNames) !== JSON.stringify([...names].sort())) {
    throw new Error(
      "mise-loaded task names differ from included task metadata",
    );
  }

  const closure = new Set();
  const visit = (name) => {
    if (closure.has(name)) return;
    closure.add(name);
    for (const dependency of taskReferences(tasks[name])) visit(dependency);
  };
  visit("check");
  for (const name of closure) {
    if (tasks[name].env.FYAGENT_TASK_EFFECT !== "read-only") {
      throw new Error(`check DAG reaches non-read-only task ${name}`);
    }
  }
  const expectedRustSequence = [
    "rust:fmt:check",
    "rust:check",
    "rust:clippy",
    "rust:test",
  ];
  if (
    JSON.stringify(sequence(tasks["check:backend"])) !==
    JSON.stringify(expectedRustSequence)
  ) {
    throw new Error("check:backend must preserve fmt/check/clippy/test order");
  }
  for (const name of ["dev", "build", "build:binary", "build:debug"]) {
    if (JSON.stringify(tasks[name]).includes("--target")) {
      throw new Error(`${name} must not accept a cross-platform target`);
    }
  }
  for (const name of ["python:run", "python:tool", "python:with"]) {
    if (tasks[name].env.FYAGENT_TASK_EFFECT !== "user-command") {
      throw new Error(
        `${name} must disclose that its arbitrary command can mutate state`,
      );
    }
  }
  const frontendRunner = read("scripts/tasks/frontend.mjs");
  if (!frontendRunner.includes("Vitest options are forbidden")) {
    throw new Error("test filters must reject write-capable Vitest options");
  }
  const rustRunner = read("scripts/tasks/rust.mjs");
  if (!rustRunner.includes("Cargo options and targets are forbidden")) {
    throw new Error("rust:test filters must reject Cargo options and targets");
  }

  const miseTrustMutation = new RegExp(
    [
      String.raw`(?:^|\s)mise(?:\.exe)?\s+`,
      String.raw`(?:trust|untrust)(?:\s|$)`,
    ].join(""),
    "i",
  );
  const forbidden = [
    [miseTrustMutation, "repository trust-state mutation"],
    [/\bgit\s+(?:push|commit|tag)(?:\s|$)/i, "Git publish/commit mutation"],
    [
      /\bgit\s+remote\s+(?:add|remove|rename|set-url|prune|update)(?:\s|$)/i,
      "Git remote mutation",
    ],
    [
      /\bgh\s+release\s+(?:create|delete|edit|upload)(?:\s|$)/i,
      "GitHub release mutation",
    ],
  ];
  for (const absolute of executableFiles()) {
    const content = fs.readFileSync(absolute, "utf8");
    for (const [pattern, label] of forbidden) {
      if (pattern.test(content)) {
        throw new Error(`${label} found in ${path.relative(ROOT, absolute)}`);
      }
    }
  }

  const config = readToml("mise.toml");
  const expectedIncludes = fs
    .readdirSync(path.join(ROOT, ".mise", "tasks"))
    .filter((name) => name.endsWith(".toml"))
    .map((name) => `.mise/tasks/${name}`)
    .sort();
  const configuredIncludes = [...(config.task_config?.includes ?? [])].sort();
  if (JSON.stringify(configuredIncludes) !== JSON.stringify(expectedIncludes)) {
    throw new Error("mise.toml task includes and .mise/tasks/*.toml differ");
  }
  if (config.settings?.task?.run_auto_install !== false) {
    throw new Error("Regular tasks must not auto-install missing tools");
  }

  for (const task of Object.values(tasks)) {
    const source = JSON.stringify(task);
    for (const match of source.matchAll(
      /scripts\/tasks\/[A-Za-z0-9_.-]+\.mjs/g,
    )) {
      if (!fs.existsSync(path.join(ROOT, match[0]))) {
        throw new Error(`Task references missing script: ${match[0]}`);
      }
    }
  }

  return { ok: true, tasks: names.length, checkClosure: [...closure].sort() };
}

if (isMain(import.meta.url)) {
  try {
    console.log(JSON.stringify(validateTaskContract(), null, 2));
  } catch (error) {
    fail(error);
  }
}
