#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const STABLE_VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const TOOL_NAMES = new Set(["node", "pnpm", "rust", "uv", "python"]);

function read(root, relativePath) {
  return fs
    .readFileSync(path.join(root, relativePath), "utf8")
    .replace(/\r\n/g, "\n");
}

function exactStableVersion(value, label) {
  const version = value.trim();
  if (!STABLE_VERSION.test(version)) {
    throw new Error(`${label} must contain one exact stable X.Y.Z version`);
  }
  return version;
}

function parseRustChannel(source) {
  let table = "";
  let toolchainTables = 0;
  const channels = [];
  for (const rawLine of source.split("\n")) {
    const line = rawLine.replace(/\s+#.*$/, "").trim();
    if (!line) continue;
    const tableMatch = /^\[([^\]]+)\]$/.exec(line);
    if (tableMatch) {
      table = tableMatch[1].trim();
      if (table === "toolchain") toolchainTables += 1;
      continue;
    }
    if (table !== "toolchain") continue;
    const channel = /^channel\s*=\s*"([^"]+)"\s*$/.exec(line);
    if (channel) channels.push(channel[1]);
  }
  if (toolchainTables !== 1 || channels.length !== 1) {
    throw new Error(
      "rust-toolchain.toml must contain exactly one [toolchain] channel",
    );
  }
  return exactStableVersion(channels[0], "rust-toolchain.toml channel");
}

function parseLockedUv(source) {
  const markers = [...source.matchAll(/^\[\[tools\.uv\]\]\s*$/gm)];
  if (markers.length !== 1) {
    throw new Error("mise.lock must contain exactly one [[tools.uv]] entry");
  }
  const start = markers[0].index + markers[0][0].length;
  const next = /^\[\[tools\.[^\]]+\]\]\s*$/gm;
  next.lastIndex = start;
  const nextMatch = next.exec(source);
  const block = source.slice(start, nextMatch?.index ?? source.length);
  const versions = [...block.matchAll(/^version\s*=\s*"([^"]+)"\s*$/gm)];
  const backends = [...block.matchAll(/^backend\s*=\s*"([^"]+)"\s*$/gm)];
  if (versions.length !== 1 || backends.length !== 1) {
    throw new Error("[[tools.uv]] must contain one version and one backend");
  }
  if (backends[0][1] !== "github:astral-sh/uv") {
    throw new Error(`Unexpected uv backend: ${backends[0][1]}`);
  }
  return exactStableVersion(versions[0][1], "mise.lock uv version");
}

export function readToolchainContract(root = ROOT) {
  const packageJson = JSON.parse(read(root, "package.json"));
  const packageManager = /^pnpm@(.+)$/.exec(packageJson.packageManager ?? "");
  if (!packageManager) {
    throw new Error("package.json#packageManager must be pnpm@X.Y.Z");
  }
  return Object.freeze({
    node: exactStableVersion(read(root, ".node-version"), ".node-version"),
    pnpm: exactStableVersion(packageManager[1], "package.json#packageManager"),
    rust: parseRustChannel(read(root, "rust-toolchain.toml")),
    python: exactStableVersion(
      read(root, ".python-version"),
      ".python-version",
    ),
    uv: parseLockedUv(read(root, "mise.lock")),
  });
}

function executable(command) {
  return process.platform === "win32" && command === "pnpm"
    ? "pnpm.cmd"
    : command;
}

function capture(command, args) {
  const result = spawnSync(executable(command), args, {
    cwd: ROOT,
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} exited with ${result.status}: ${(result.stderr || result.stdout || "").trim()}`,
    );
  }
  return (result.stdout || "").trim();
}

function expectVersion(name, actual, expected) {
  if (actual !== expected) {
    throw new Error(`${name}: expected ${expected}, got ${actual}`);
  }
  return actual;
}

export function verifyInstalledToolchains(contract, tools) {
  const verified = {};
  for (const tool of tools) {
    if (!TOOL_NAMES.has(tool)) throw new Error(`Unknown tool name: ${tool}`);
    if (tool === "node") {
      verified.node = expectVersion(
        "node",
        process.versions.node,
        contract.node,
      );
    } else if (tool === "pnpm") {
      verified.pnpm = expectVersion(
        "pnpm",
        capture("pnpm", ["--version"]),
        contract.pnpm,
      );
    } else if (tool === "rust") {
      const verbose = capture("rustc", ["--version", "--verbose"]);
      const release = /^release:\s*(\S+)$/m.exec(verbose)?.[1];
      verified.rust = expectVersion("rustc", release, contract.rust);
      verified.rustfmt = capture("rustfmt", ["--version"]);
      verified.clippy = capture("cargo", ["clippy", "--version"]);
    } else if (tool === "uv") {
      const actual = /^uv\s+(\S+)/.exec(capture("uv", ["--version"]))?.[1];
      verified.uv = expectVersion("uv", actual, contract.uv);
    } else if (tool === "python") {
      const actual = capture("uv", [
        "run",
        "--locked",
        "--no-sync",
        "python",
        "-c",
        "import platform; print(platform.python_version())",
      ]);
      verified.python = expectVersion("Python", actual, contract.python);
    }
  }
  return verified;
}

export function writeGithubOutputs(contract, outputPath) {
  if (!outputPath) throw new Error("GITHUB_OUTPUT is required");
  fs.appendFileSync(
    outputPath,
    [
      `node-version=${contract.node}`,
      `pnpm-version=${contract.pnpm}`,
      `rust-version=${contract.rust}`,
      `python-version=${contract.python}`,
      `uv-version=${contract.uv}`,
      "",
    ].join("\n"),
  );
}

function parseTools(argv) {
  const index = argv.indexOf("--tools");
  if (index < 0) return [...TOOL_NAMES];
  const value = argv[index + 1];
  if (!value) throw new Error("--tools requires a comma-separated value");
  const tools = value.split(",").filter(Boolean);
  if (tools.length === 0 || new Set(tools).size !== tools.length) {
    throw new Error("--tools must name unique tools");
  }
  for (const tool of tools) {
    if (!TOOL_NAMES.has(tool)) throw new Error(`Unknown tool name: ${tool}`);
  }
  return tools;
}

export function runToolchainCli(
  argv = process.argv.slice(2),
  env = process.env,
) {
  try {
    const contract = readToolchainContract();
    if (argv.includes("--emit-github-output")) {
      writeGithubOutputs(contract, env.GITHUB_OUTPUT);
      console.log(JSON.stringify({ ok: true, contract }, null, 2));
      return { ok: true, contract, verified: {} };
    }
    const verified = verifyInstalledToolchains(contract, parseTools(argv));
    const report = { ok: true, contract, verified };
    console.log(JSON.stringify(report, null, 2));
    return report;
  } catch (error) {
    const report = {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
    console.error(JSON.stringify(report, null, 2));
    process.exitCode = 1;
    return report;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  runToolchainCli();
}
