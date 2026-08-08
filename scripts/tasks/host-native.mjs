#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { ROOT, capture, fail, isMain, run } from "./lib.mjs";
import { parse as parseToml } from "smol-toml";

export const HOST_RUST_TARGETS = Object.freeze({
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
});

const TAURI_OPERATIONS = Object.freeze({
  dev: Object.freeze(["dev"]),
  build: Object.freeze(["build"]),
  "build:binary": Object.freeze(["build", "--no-bundle"]),
  "build:debug": Object.freeze(["build", "--debug"]),
});

const CARGO_OPERATIONS = new Set(["check", "clippy", "test"]);
const OWNED_TOOLCHAIN_ENVIRONMENT = Object.freeze([
  "CARGO_BUILD_TARGET",
  "TAURI_ENV_TARGET_TRIPLE",
  "RUSTC",
  "CARGO_BUILD_RUSTC",
  "RUSTC_WRAPPER",
  "CARGO_BUILD_RUSTC_WRAPPER",
  "RUSTC_WORKSPACE_WRAPPER",
  "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
  "RUSTDOC",
  "CARGO_BUILD_RUSTDOC",
]);
const RUST_FLAG_ENVIRONMENT = Object.freeze([
  "RUSTFLAGS",
  "CARGO_BUILD_RUSTFLAGS",
  "CARGO_ENCODED_RUSTFLAGS",
]);
const RUSTDOC_FLAG_ENVIRONMENT = Object.freeze([
  "RUSTDOCFLAGS",
  "CARGO_BUILD_RUSTDOCFLAGS",
  "CARGO_ENCODED_RUSTDOCFLAGS",
]);
const TARGET_RUNNER_ENVIRONMENT = /^CARGO_TARGET_.+_RUNNER$/;
const TARGET_LINKER_ENVIRONMENT = /^CARGO_TARGET_.+_LINKER$/;
const TARGET_RUST_FLAG_ENVIRONMENT = /^CARGO_TARGET_.+_RUSTFLAGS$/;
const TARGET_RUSTDOC_FLAG_ENVIRONMENT = /^CARGO_TARGET_.+_RUSTDOCFLAGS$/;
const PROCESS_INJECTION_ENVIRONMENT = Object.freeze([
  "LD_PRELOAD",
  "LD_LIBRARY_PATH",
  "DYLD_INSERT_LIBRARIES",
  "DYLD_FORCE_FLAT_NAMESPACE",
  "DYLD_LIBRARY_PATH",
  "DYLD_FRAMEWORK_PATH",
  "DYLD_FALLBACK_LIBRARY_PATH",
  "NODE_OPTIONS",
  "NODE_PATH",
]);

function environmentEntries(environment) {
  return Object.entries(environment ?? {}).filter(
    ([, value]) => value !== undefined,
  );
}

function environmentValue(environment, name) {
  const matches = environmentEntries(environment).filter(
    ([candidate]) => candidate.toUpperCase() === name.toUpperCase(),
  );
  if (matches.length > 1) {
    throw new Error(`Ambiguous case variants are set for ${name}`);
  }
  return matches[0]?.[1];
}

export function expectedRustTarget(platform, architecture) {
  const key = `${platform}-${architecture}`;
  const target = HOST_RUST_TARGETS[key];
  if (!target) {
    throw new Error(
      `Unsupported local host OS/architecture: ${platform}/${architecture}`,
    );
  }
  return target;
}

export function parseRustcHost(verboseVersion) {
  const hosts = String(verboseVersion)
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => /^host:\s*(\S+)\s*$/.exec(line)?.[1])
    .filter(Boolean);
  if (hosts.length !== 1) {
    throw new Error("rustc -vV must report exactly one host target");
  }
  return hosts[0];
}

export function parseToolIdentity(verboseVersion, tool) {
  const normalized = String(verboseVersion).replace(/\r\n/g, "\n");
  const uniqueValue = (label) => {
    const values = normalized
      .split("\n")
      .map((line) => new RegExp(`^${label}:\\s*(\\S+)\\s*$`).exec(line)?.[1])
      .filter(Boolean);
    if (values.length !== 1) {
      throw new Error(`${tool} -vV must report exactly one ${label}`);
    }
    return values[0];
  };
  return {
    host: uniqueValue("host"),
    release: uniqueValue("release"),
    commitHash: uniqueValue("commit-hash"),
  };
}

export function assertCurrentRustHost({
  platform,
  architecture,
  rustcVerboseVersion,
}) {
  const expected = expectedRustTarget(platform, architecture);
  const actual = parseRustcHost(rustcVerboseVersion);
  if (actual !== expected) {
    throw new Error(
      `rustc host ${actual} does not match current host ${platform}/${architecture} (${expected})`,
    );
  }
  return actual;
}

export function assertCurrentToolchain({
  platform,
  architecture,
  rustcVerboseVersion,
  rustdocVerboseVersion,
}) {
  const target = assertCurrentRustHost({
    platform,
    architecture,
    rustcVerboseVersion,
  });
  const rustc = parseToolIdentity(rustcVerboseVersion, "rustc");
  const rustdoc = parseToolIdentity(rustdocVerboseVersion, "rustdoc");
  if (
    rustdoc.host !== target ||
    rustdoc.release !== rustc.release ||
    rustdoc.commitHash !== rustc.commitHash
  ) {
    throw new Error(
      `rustdoc ${rustdoc.release}/${rustdoc.commitHash}/${rustdoc.host} does not match rustc ${rustc.release}/${rustc.commitHash}/${target}`,
    );
  }
  return target;
}

export function containsRustTargetOverride(value, encoded = false) {
  if (typeof value !== "string" || value === "") return false;
  const tokens = encoded ? value.split("\u001f") : value.trim().split(/\s+/);
  return tokens.some(
    (token) => token === "--target" || token.startsWith("--target="),
  );
}

export function assertNoCallerTargetOverride(environment) {
  const owned = new Set(OWNED_TOOLCHAIN_ENVIRONMENT);
  const processInjection = new Set(PROCESS_INJECTION_ENVIRONMENT);
  const rustFlags = new Set(RUST_FLAG_ENVIRONMENT);
  const rustdocFlags = new Set(RUSTDOC_FLAG_ENVIRONMENT);
  for (const [name, value] of environmentEntries(environment)) {
    const normalizedName = name.toUpperCase();
    if (owned.has(normalizedName)) {
      throw new Error(
        `${name} must not be set; canonical local tasks own the current-host compiler, wrappers, and target`,
      );
    }
    if (processInjection.has(normalizedName)) {
      throw new Error(
        `${name} must not be set; canonical local tasks reject process loader and runtime injection controls`,
      );
    }
    if (TARGET_RUNNER_ENVIRONMENT.test(normalizedName)) {
      throw new Error(
        `${name} must not be set; canonical local tasks own the current-host test runner`,
      );
    }
    if (TARGET_LINKER_ENVIRONMENT.test(normalizedName)) {
      throw new Error(
        `${name} must not be set; canonical local tasks own the current-host linker`,
      );
    }
    const isRustFlags =
      rustFlags.has(normalizedName) ||
      TARGET_RUST_FLAG_ENVIRONMENT.test(normalizedName);
    const isRustdocFlags =
      rustdocFlags.has(normalizedName) ||
      TARGET_RUSTDOC_FLAG_ENVIRONMENT.test(normalizedName);
    if (
      TARGET_RUST_FLAG_ENVIRONMENT.test(normalizedName) ||
      TARGET_RUSTDOC_FLAG_ENVIRONMENT.test(normalizedName)
    ) {
      throw new Error(
        `${name} must not be set; canonical local tasks reject target-specific compiler and linker flags`,
      );
    }
    if (
      (isRustFlags || isRustdocFlags) &&
      containsRustTargetOverride(value, normalizedName.includes("ENCODED_RUST"))
    ) {
      throw new Error(
        `${name} must not contain --target; canonical local tasks own the current-host target`,
      );
    }
  }
}

export function resolveToolExecutable({
  tool,
  environment = process.env,
  platform = process.platform,
  cwd = process.cwd(),
}) {
  const searchPath = environmentValue(environment, "PATH");
  if (typeof searchPath !== "string" || searchPath === "") {
    throw new Error(`PATH is required to resolve the canonical ${tool}`);
  }
  const pathApi = platform === "win32" ? path.win32 : path.posix;
  const delimiter = platform === "win32" ? ";" : ":";
  const executable = platform === "win32" ? `${tool}.exe` : tool;
  for (const rawDirectory of searchPath.split(delimiter)) {
    const unquoted = rawDirectory.replace(/^"(.*)"$/, "$1");
    const directory = unquoted === "" ? cwd : unquoted;
    const candidate = pathApi.resolve(directory, executable);
    try {
      if (!fs.statSync(candidate).isFile()) continue;
      if (platform !== "win32") {
        fs.accessSync(candidate, fs.constants.X_OK);
      }
      return candidate;
    } catch {
      // Continue through PATH exactly as process spawning would.
    }
  }
  throw new Error(`Unable to resolve ${tool} to an executable in PATH`);
}

export function buildNativeRunnerConfig({
  target,
  platform,
  nodeExecutable = process.execPath,
  runnerScript = fileURLToPath(import.meta.url),
}) {
  const pathApi = platform === "win32" ? path.win32 : path.posix;
  for (const [label, executable] of [
    ["Node", nodeExecutable],
    ["host-native runner", runnerScript],
  ]) {
    if (typeof executable !== "string" || !pathApi.isAbsolute(executable)) {
      throw new Error(`${label} must be an absolute path`);
    }
  }
  return `target.${target}.runner=${JSON.stringify([
    nodeExecutable,
    runnerScript,
    "native-runner",
    target,
  ])}`;
}

export function resolveNativeRunner({
  target,
  platform = process.platform,
  nodeExecutable = process.execPath,
  runnerScript = fileURLToPath(import.meta.url),
}) {
  for (const [label, executable] of [
    ["Node", nodeExecutable],
    ["host-native runner", runnerScript],
  ]) {
    try {
      const stat = fs.lstatSync(executable);
      if (!stat.isFile() || stat.isSymbolicLink())
        throw new Error("not a file");
    } catch {
      throw new Error(`${label} is not a regular non-symlink file`);
    }
  }
  return buildNativeRunnerConfig({
    target,
    platform,
    nodeExecutable,
    runnerScript,
  });
}

function cargoConfigCandidates({ root, environment }) {
  const result = [];
  let directory = path.resolve(root);
  while (true) {
    result.push(
      path.join(directory, ".cargo", "config"),
      path.join(directory, ".cargo", "config.toml"),
    );
    const parent = path.dirname(directory);
    if (parent === directory) break;
    directory = parent;
  }
  const configuredHome = environmentValue(environment, "CARGO_HOME");
  const cargoHome =
    typeof configuredHome === "string" && configuredHome !== ""
      ? path.resolve(root, configuredHome)
      : path.join(os.homedir(), ".cargo");
  result.push(
    path.join(cargoHome, "config"),
    path.join(cargoHome, "config.toml"),
  );
  return [...new Set(result)];
}

function cargoConfigIncludes(config, configFile) {
  if (!Array.isArray(config.include)) return [];
  return config.include.map((entry) => {
    if (typeof entry === "string") {
      return { path: entry, optional: false };
    }
    if (entry && typeof entry === "object" && typeof entry.path === "string") {
      return { path: entry.path, optional: entry.optional === true };
    }
    throw new Error(`Invalid Cargo include entry in ${configFile}`);
  });
}

const FORBIDDEN_CARGO_BUILD_KEYS = Object.freeze([
  "target",
  "rustc",
  "rustc-wrapper",
  "rustc-workspace-wrapper",
  "rustdoc",
  "rustflags",
  "rustdocflags",
]);
const FORBIDDEN_CARGO_TARGET_KEYS = Object.freeze([
  "runner",
  "linker",
  "rustflags",
  "rustdocflags",
]);
const MAX_CARGO_CONFIG_FILES = 128;
const MAX_CARGO_INCLUDE_DEPTH = 32;

function isForbiddenCargoEnvironmentName(name) {
  const normalized = name.toUpperCase();
  return (
    OWNED_TOOLCHAIN_ENVIRONMENT.includes(normalized) ||
    PROCESS_INJECTION_ENVIRONMENT.includes(normalized) ||
    RUST_FLAG_ENVIRONMENT.includes(normalized) ||
    RUSTDOC_FLAG_ENVIRONMENT.includes(normalized) ||
    TARGET_RUNNER_ENVIRONMENT.test(normalized) ||
    TARGET_LINKER_ENVIRONMENT.test(normalized) ||
    TARGET_RUST_FLAG_ENVIRONMENT.test(normalized) ||
    TARGET_RUSTDOC_FLAG_ENVIRONMENT.test(normalized)
  );
}

function assertNoForbiddenCargoSettings(config, configFile) {
  if (config.build && typeof config.build === "object") {
    for (const key of FORBIDDEN_CARGO_BUILD_KEYS) {
      if (Object.prototype.hasOwnProperty.call(config.build, key)) {
        throw new Error(
          `Cargo config build.${key} is forbidden in canonical local tasks (${configFile})`,
        );
      }
    }
  }
  if (config.target && typeof config.target === "object") {
    for (const [target, settings] of Object.entries(config.target)) {
      if (!settings || typeof settings !== "object") continue;
      for (const key of FORBIDDEN_CARGO_TARGET_KEYS) {
        if (Object.prototype.hasOwnProperty.call(settings, key)) {
          throw new Error(
            `Cargo config target.${target}.${key} is forbidden in canonical local tasks (${configFile})`,
          );
        }
      }
    }
  }
  if (config.env && typeof config.env === "object") {
    for (const name of Object.keys(config.env)) {
      if (isForbiddenCargoEnvironmentName(name)) {
        throw new Error(
          `Cargo config env.${name} is forbidden in canonical local tasks (${configFile})`,
        );
      }
    }
  }
}

export function assertNoCargoToolchainConfig({
  root = ROOT,
  environment = process.env,
}) {
  const completed = new Set();
  const active = new Set();
  const visit = (configFile, optional = false, depth = 0) => {
    if (depth > MAX_CARGO_INCLUDE_DEPTH) {
      throw new Error(
        `Cargo config include depth exceeds ${MAX_CARGO_INCLUDE_DEPTH}`,
      );
    }
    const absolute = path.resolve(configFile);
    if (!fs.existsSync(absolute)) {
      if (optional) return;
      throw new Error(
        `Required Cargo config include does not exist: ${absolute}`,
      );
    }
    const stat = fs.lstatSync(absolute);
    if (!stat.isFile() || stat.isSymbolicLink()) {
      throw new Error(
        `Cargo config must be a regular non-symlink file: ${absolute}`,
      );
    }
    const real = fs.realpathSync(absolute);
    if (active.has(real)) {
      throw new Error(`Cargo config include cycle is forbidden: ${real}`);
    }
    if (completed.has(real)) return;
    if (completed.size + active.size >= MAX_CARGO_CONFIG_FILES) {
      throw new Error(
        `Cargo config graph exceeds ${MAX_CARGO_CONFIG_FILES} files`,
      );
    }
    active.add(real);
    let config;
    try {
      config = parseToml(fs.readFileSync(real, "utf8"));
    } catch (error) {
      throw new Error(
        `Unable to inspect Cargo config ${real}: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
    assertNoForbiddenCargoSettings(config, real);
    for (const include of cargoConfigIncludes(config, real)) {
      visit(
        path.resolve(path.dirname(real), include.path),
        include.optional,
        depth + 1,
      );
    }
    active.delete(real);
    completed.add(real);
  };
  for (const candidate of cargoConfigCandidates({ root, environment })) {
    visit(candidate, true);
  }
}

export function ownedCargoEnvironment({
  rustcExecutable,
  rustdocExecutable,
  platform,
}) {
  const pathApi = platform === "win32" ? path.win32 : path.posix;
  for (const [tool, executable] of [
    ["rustc", rustcExecutable],
    ["rustdoc", rustdocExecutable],
  ]) {
    if (typeof executable !== "string" || !pathApi.isAbsolute(executable)) {
      throw new Error(`${tool} must resolve to an absolute executable path`);
    }
  }
  return {
    RUSTC: rustcExecutable,
    CARGO_BUILD_RUSTC: rustcExecutable,
    RUSTC_WRAPPER: "",
    CARGO_BUILD_RUSTC_WRAPPER: "",
    RUSTC_WORKSPACE_WRAPPER: "",
    CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER: "",
    RUSTDOC: rustdocExecutable,
    CARGO_BUILD_RUSTDOC: rustdocExecutable,
    RUSTFLAGS: "",
    CARGO_BUILD_RUSTFLAGS: "",
    CARGO_ENCODED_RUSTFLAGS: "",
    RUSTDOCFLAGS: "",
    CARGO_BUILD_RUSTDOCFLAGS: "",
    CARGO_ENCODED_RUSTDOCFLAGS: "",
    ...Object.fromEntries(
      PROCESS_INJECTION_ENVIRONMENT.map((name) => [name, ""]),
    ),
  };
}

function samePath(left, right, platform) {
  const normalize = (value) =>
    platform === "win32" ? value.toLowerCase() : value;
  return normalize(path.normalize(left)) === normalize(path.normalize(right));
}

function isPathWithin(parent, candidate, platform) {
  const relative = path.relative(parent, candidate);
  if (relative === "" || relative === ".") return false;
  const normalized = platform === "win32" ? relative.toLowerCase() : relative;
  return !normalized.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

function expectedNativeMachine(target) {
  if (target.startsWith("x86_64-")) {
    return { elf: 62, pe: 0x8664, macho: 0x01000007 };
  }
  if (target.startsWith("aarch64-")) {
    return { elf: 183, pe: 0xaa64, macho: 0x0100000c };
  }
  throw new Error(`Unsupported native executable target: ${target}`);
}

function verifyNativeBinarySignature(file, platform, target) {
  const expectedMachine = expectedNativeMachine(target);
  const handle = fs.openSync(file, "r");
  try {
    const header = Buffer.alloc(64);
    const length = fs.readSync(handle, header, 0, header.length, 0);
    if (platform === "linux") {
      if (
        length < 20 ||
        !header.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))
      ) {
        throw new Error("Native Linux test executable must have an ELF header");
      }
      if (header[4] !== 2 || header[5] !== 1) {
        throw new Error(
          "Native Linux test executable must be a little-endian 64-bit ELF",
        );
      }
      const machine = header.readUInt16LE(18);
      if (machine !== expectedMachine.elf) {
        throw new Error(
          `Native Linux test executable architecture ${machine} does not match ${target}`,
        );
      }
      return;
    }
    if (platform === "darwin") {
      if (length < 8) {
        throw new Error(
          "Native macOS test executable must have a Mach-O header",
        );
      }
      const magic = header.readUInt32BE(0);
      let machine;
      if (magic === 0xfeedfacf) {
        machine = header.readUInt32BE(4);
      } else if (magic === 0xcffaedfe) {
        machine = header.readUInt32LE(4);
      } else {
        throw new Error(
          "Native macOS test executable must be a thin 64-bit Mach-O",
        );
      }
      if (machine !== expectedMachine.macho) {
        throw new Error(
          `Native macOS test executable architecture ${machine} does not match ${target}`,
        );
      }
      return;
    }
    if (platform === "win32") {
      if (length < 64 || header[0] !== 0x4d || header[1] !== 0x5a) {
        throw new Error("Native Windows test executable must have a PE header");
      }
      const peOffset = header.readUInt32LE(0x3c);
      const pe = Buffer.alloc(6);
      if (
        fs.readSync(handle, pe, 0, pe.length, peOffset) !== pe.length ||
        !pe.subarray(0, 4).equals(Buffer.from([0x50, 0x45, 0x00, 0x00]))
      ) {
        throw new Error("Native Windows test executable has no PE signature");
      }
      const machine = pe.readUInt16LE(4);
      if (machine !== expectedMachine.pe) {
        throw new Error(
          `Native Windows test executable architecture ${machine} does not match ${target}`,
        );
      }
      return;
    }
    throw new Error(`Unsupported local executable platform: ${platform}`);
  } finally {
    fs.closeSync(handle);
  }
}

export function verifyNativeTestExecutable({
  target,
  executable,
  platform = process.platform,
  architecture = process.arch,
  cwd = process.cwd(),
  root = ROOT,
}) {
  const expected = expectedRustTarget(platform, architecture);
  if (target !== expected) {
    throw new Error(
      `Native runner target ${target ?? ""} does not match current host ${expected}`,
    );
  }
  if (typeof executable !== "string" || executable === "") {
    throw new Error("Cargo did not provide a native test executable");
  }
  const targetDirectory = path.resolve(root, "src-tauri", "target", expected);
  const absolute = path.resolve(cwd, executable);
  const targetStat = fs.lstatSync(targetDirectory);
  if (!targetStat.isDirectory() || targetStat.isSymbolicLink()) {
    throw new Error(
      "Current-host Cargo target directory must not be a symlink",
    );
  }
  const realTarget = fs.realpathSync(targetDirectory);
  if (!samePath(realTarget, targetDirectory, platform)) {
    throw new Error(
      "Current-host Cargo target directory leaves the repository",
    );
  }
  const stat = fs.lstatSync(absolute);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error("Cargo test executable must be a regular non-symlink file");
  }
  const realExecutable = fs.realpathSync(absolute);
  if (!isPathWithin(realTarget, realExecutable, platform)) {
    throw new Error(
      "Cargo test executable must stay inside the verified current-host target directory",
    );
  }
  verifyNativeBinarySignature(realExecutable, platform, expected);
  return realExecutable;
}

export function executeNativeTest({
  target,
  executable,
  arguments: testArguments = [],
  platform = process.platform,
  architecture = process.arch,
}) {
  const verified = verifyNativeTestExecutable({
    target,
    executable,
    platform,
    architecture,
  });
  const result = spawnSync(verified, testArguments, {
    cwd: ROOT,
    env: {
      ...process.env,
      ...Object.fromEntries(
        PROCESS_INJECTION_ENVIRONMENT.map((name) => [name, ""]),
      ),
    },
    stdio: "inherit",
    shell: false,
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.signal) {
    throw new Error(`Native test executable terminated by ${result.signal}`);
  }
  return result.status ?? 1;
}

function assertNoForwardedArguments(operation, forwardedArguments) {
  if (forwardedArguments.length > 0) {
    throw new Error(
      `${operation} does not accept forwarded arguments; canonical local tasks use a fixed current-host operation`,
    );
  }
}

export function assertHostNativeGuard({
  forwardedArguments = [],
  environment = process.env,
  platform = process.platform,
  architecture = process.arch,
}) {
  assertNoForwardedArguments("host-native guard", forwardedArguments);
  assertNoCallerTargetOverride(environment);
  return expectedRustTarget(platform, architecture);
}

export function assertTauriRequest({
  operation,
  forwardedArguments = [],
  environment,
}) {
  if (!Object.hasOwn(TAURI_OPERATIONS, operation)) {
    throw new Error(`Unknown native Tauri operation: ${operation ?? ""}`);
  }
  assertNoForwardedArguments(operation, forwardedArguments);
  assertNoCallerTargetOverride(environment);
}

export function planTauriTask({
  operation,
  forwardedArguments = [],
  environment,
  platform,
  architecture,
  rustcVerboseVersion,
  rustdocVerboseVersion,
  rustcExecutable,
  rustdocExecutable,
}) {
  assertTauriRequest({ operation, forwardedArguments, environment });
  const target = assertCurrentToolchain({
    platform,
    architecture,
    rustcVerboseVersion,
    rustdocVerboseVersion,
  });
  const [subcommand, ...fixedArguments] = TAURI_OPERATIONS[operation];
  return {
    command: "pnpm",
    args: ["tauri", subcommand, "--target", target, ...fixedArguments],
    target,
    environment: ownedCargoEnvironment({
      rustcExecutable,
      rustdocExecutable,
      platform,
    }),
  };
}

export function assertCargoRequest({
  operation,
  filters = [],
  forwardedArguments = [],
  environment,
}) {
  if (!CARGO_OPERATIONS.has(operation)) {
    throw new Error(`Unknown Rust task command: ${operation ?? ""}`);
  }
  assertNoForwardedArguments(`rust:${operation}`, forwardedArguments);
  assertNoCallerTargetOverride(environment);
  if (operation !== "test" && filters.length > 0) {
    throw new Error(`rust:${operation} does not accept test-name filters`);
  }
  if (
    filters.some(
      (filter) => filter.startsWith("-") || filter.includes("--target"),
    )
  ) {
    throw new Error(
      "rust:test accepts test-name filters only; Cargo options and targets are forbidden",
    );
  }
  if (filters.length > 1) {
    throw new Error("rust:test accepts at most one test-name filter");
  }
}

export function planCargoTask({
  operation,
  filters = [],
  forwardedArguments = [],
  environment,
  platform,
  architecture,
  rustcVerboseVersion,
  rustdocVerboseVersion,
  rustcExecutable,
  rustdocExecutable,
  nativeRunnerConfig,
}) {
  assertCargoRequest({
    operation,
    filters,
    forwardedArguments,
    environment,
  });
  const target = assertCurrentToolchain({
    platform,
    architecture,
    rustcVerboseVersion,
    rustdocVerboseVersion,
  });
  const commonArguments = [
    "--workspace",
    "--locked",
    "--manifest-path",
    "src-tauri/Cargo.toml",
  ];
  let args;
  if (operation === "check") {
    args = [
      "--config",
      nativeRunnerConfig,
      "check",
      "--target",
      target,
      ...commonArguments,
      "--all-targets",
    ];
  } else if (operation === "clippy") {
    args = [
      "--config",
      nativeRunnerConfig,
      "clippy",
      "--target",
      target,
      ...commonArguments,
      "--all-targets",
      "--",
      "-D",
      "warnings",
    ];
  } else {
    args = [
      "--config",
      nativeRunnerConfig,
      "test",
      "--target",
      target,
      ...commonArguments,
      ...(filters.length === 1 ? ["--", filters[0]] : []),
    ];
  }
  return {
    command: "cargo",
    args,
    target,
    environment: ownedCargoEnvironment({
      rustcExecutable,
      rustdocExecutable,
      platform,
    }),
  };
}

export function executeTauriTask({
  operation,
  forwardedArguments = [],
  environment = process.env,
  platform = process.platform,
  architecture = process.arch,
  captureCommand = capture,
  runCommand = run,
  resolveToolCommand = resolveToolExecutable,
}) {
  assertTauriRequest({ operation, forwardedArguments, environment });
  assertNoCargoToolchainConfig({ environment });
  const rustcExecutable = resolveToolCommand({
    tool: "rustc",
    environment,
    platform,
  });
  const rustdocExecutable = resolveToolCommand({
    tool: "rustdoc",
    environment,
    platform,
  });
  const rustcVerboseVersion = captureCommand(rustcExecutable, ["-vV"]);
  const rustdocVerboseVersion = captureCommand(rustdocExecutable, ["-vV"]);
  const plan = planTauriTask({
    operation,
    forwardedArguments,
    environment,
    platform,
    architecture,
    rustcVerboseVersion,
    rustdocVerboseVersion,
    rustcExecutable,
    rustdocExecutable,
  });
  runCommand(plan.command, plan.args, { env: plan.environment });
  return plan;
}

export function executeCargoTask({
  operation,
  filters = [],
  forwardedArguments = [],
  environment = process.env,
  platform = process.platform,
  architecture = process.arch,
  captureCommand = capture,
  runCommand = run,
  resolveToolCommand = resolveToolExecutable,
  resolveRunner = resolveNativeRunner,
}) {
  assertCargoRequest({
    operation,
    filters,
    forwardedArguments,
    environment,
  });
  assertNoCargoToolchainConfig({ environment });
  const rustcExecutable = resolveToolCommand({
    tool: "rustc",
    environment,
    platform,
  });
  const rustdocExecutable = resolveToolCommand({
    tool: "rustdoc",
    environment,
    platform,
  });
  const nativeRunnerConfig = resolveRunner({
    environment,
    platform,
    target: expectedRustTarget(platform, architecture),
  });
  const rustcVerboseVersion = captureCommand(rustcExecutable, ["-vV"]);
  const rustdocVerboseVersion = captureCommand(rustdocExecutable, ["-vV"]);
  const plan = planCargoTask({
    operation,
    filters,
    forwardedArguments,
    environment,
    platform,
    architecture,
    rustcVerboseVersion,
    rustdocVerboseVersion,
    rustcExecutable,
    rustdocExecutable,
    nativeRunnerConfig,
  });
  runCommand(plan.command, plan.args, { env: plan.environment });
  return plan;
}

function main() {
  if (process.argv[2] === "native-runner") {
    process.exitCode = executeNativeTest({
      target: process.argv[3],
      executable: process.argv[4],
      arguments: process.argv.slice(5),
    });
    return;
  }
  if (process.argv[2] === "guard") {
    assertHostNativeGuard({
      forwardedArguments: process.argv.slice(3),
      environment: process.env,
      platform: process.platform,
      architecture: process.arch,
    });
    return;
  }
  executeTauriTask({
    operation: process.argv[2],
    forwardedArguments: process.argv.slice(3),
  });
}

if (isMain(import.meta.url)) {
  try {
    main();
  } catch (error) {
    fail(error);
  }
}
