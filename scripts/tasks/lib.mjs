#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parse as parseToml } from "smol-toml";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));

export const ROOT = path.resolve(scriptDirectory, "..", "..");
export const SUPPORTED_PLATFORMS = Object.freeze([
  "linux-x64",
  "linux-arm64",
  "macos-x64",
  "macos-arm64",
  "windows-x64",
  "windows-arm64",
]);
export const STABLE_SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export function resolveTaskExecutable(command, platform = process.platform) {
  return platform === "win32" && command === "pnpm" ? "pnpm.exe" : command;
}

export function run(command, args = [], options = {}) {
  const result = spawnSync(resolveTaskExecutable(command), args, {
    cwd: ROOT,
    env: { ...process.env, ...(options.env ?? {}) },
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0 && !options.allowFailure) {
    const detail = options.capture
      ? `\n${(result.stderr || result.stdout || "").trim()}`
      : "";
    throw new Error(
      `${command} ${args.join(" ")} exited with ${result.status}${detail}`,
    );
  }
  return {
    status: result.status ?? 1,
    stdout: (result.stdout ?? "").trim(),
    stderr: (result.stderr ?? "").trim(),
  };
}

export function capture(command, args = [], options = {}) {
  return run(command, args, { ...options, capture: true }).stdout;
}

export function read(relativePath) {
  return fs
    .readFileSync(path.join(ROOT, relativePath), "utf8")
    .replace(/\r\n/g, "\n");
}

export function readToml(relativePath) {
  return parseToml(read(relativePath));
}

export function readJson(relativePath) {
  return JSON.parse(read(relativePath));
}

export function usageBoolean(name) {
  return /^(1|true|yes)$/i.test(process.env[`usage_${name}`] ?? "");
}

export function usageValue(name) {
  const value = process.env[`usage_${name}`];
  return value === undefined || value === "" ? undefined : value;
}

// mise serializes variadic usage values as a shell-escaped string. Parse only
// quoting and escaping here; commands are always spawned with an argv array.
export function usageList(name) {
  const input = usageValue(name);
  if (!input) return [];
  const output = [];
  let current = "";
  let quote = null;
  let escaped = false;
  let started = false;
  for (const character of input) {
    if (escaped) {
      current += character;
      escaped = false;
      started = true;
      continue;
    }
    if (character === "\\" && quote !== "'") {
      escaped = true;
      started = true;
      continue;
    }
    if (quote) {
      if (character === quote) quote = null;
      else current += character;
      started = true;
      continue;
    }
    if (character === "'" || character === '"') {
      quote = character;
      started = true;
      continue;
    }
    if (/\s/.test(character)) {
      if (started) {
        output.push(current);
        current = "";
        started = false;
      }
      continue;
    }
    current += character;
    started = true;
  }
  if (escaped || quote) throw new Error(`Malformed usage_${name} value`);
  if (started) output.push(current);
  return output;
}

export function assertStableSemver(value, label = "version") {
  if (!value || !STABLE_SEMVER.test(value)) {
    throw new Error(`${label} must be an exact stable X.Y.Z version`);
  }
  return value;
}

export function assertSimplePackageNames(values, label = "package") {
  if (values.length === 0) throw new Error(`At least one ${label} is required`);
  for (const value of values) {
    if (!/^(?:@[-a-z0-9_.]+\/)?[-a-z0-9_.]+(?:@[^\s]+)?$/i.test(value)) {
      throw new Error(`Invalid ${label}: ${value}`);
    }
  }
  return values;
}

export function repositoryPath(relativePath) {
  const absolute = path.resolve(ROOT, relativePath);
  const relative = path.relative(ROOT, absolute);
  if (
    relative === "" ||
    relative === "." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error(`Path must be a child of the repository: ${relativePath}`);
  }
  return absolute;
}

export function writeFilesAtomically(changes) {
  const originals = new Map();
  const temporary = [];
  try {
    for (const [relativePath, content] of changes) {
      const absolute = repositoryPath(relativePath);
      originals.set(
        absolute,
        fs.existsSync(absolute) ? fs.readFileSync(absolute) : null,
      );
      const temporaryPath = `${absolute}.fyagent-task-${process.pid}.tmp`;
      fs.writeFileSync(temporaryPath, content, { flag: "wx" });
      temporary.push([temporaryPath, absolute]);
    }
    for (const [temporaryPath, absolute] of temporary) {
      fs.renameSync(temporaryPath, absolute);
    }
  } catch (error) {
    for (const [temporaryPath] of temporary) {
      fs.rmSync(temporaryPath, { force: true });
    }
    for (const [absolute, original] of originals) {
      if (original === null) fs.rmSync(absolute, { force: true });
      else fs.writeFileSync(absolute, original);
    }
    throw error;
  }
}

export function isMain(importMetaUrl) {
  if (!process.argv[1]) return false;
  return pathToFileURL(path.resolve(process.argv[1])).href === importMetaUrl;
}

export function printPlan(title, command, args) {
  console.log(
    JSON.stringify(
      { status: "preview", title, command: [command, ...args] },
      null,
      2,
    ),
  );
}

export function fail(error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
