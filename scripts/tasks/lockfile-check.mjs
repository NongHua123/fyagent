#!/usr/bin/env node

import process from "node:process";
import {
  SUPPORTED_PLATFORMS,
  fail,
  isMain,
  read,
  readJson,
  readToml,
} from "./lib.mjs";

const EXPECTED = Object.freeze({
  node: "24.19.0",
  pnpm: "10.12.3",
  rust: "1.97.1",
});

const ARCH_MARKERS = Object.freeze({
  "linux-x64": /(?:x64|x86_64)/i,
  "linux-arm64": /(?:arm64|aarch64)/i,
  "macos-x64": /(?:x64|x86_64)/i,
  "macos-arm64": /(?:arm64|aarch64)/i,
  "windows-x64": /(?:x64|x86_64)/i,
  "windows-arm64": /(?:arm64|aarch64)/i,
});

function entriesFor(lock, name) {
  const entries = lock.tools?.[name];
  if (!Array.isArray(entries) || entries.length !== 1) {
    throw new Error(`mise.lock must contain exactly one ${name} entry`);
  }
  return entries[0];
}

function validateArtifactPlatforms(name, entry) {
  for (const platform of SUPPORTED_PLATFORMS) {
    const artifact = entry[`platforms.${platform}`];
    if (!artifact) throw new Error(`${name} is not locked for ${platform}`);
    if (!/^sha256:[a-f0-9]{64}$/.test(artifact.checksum ?? "")) {
      throw new Error(`${name} ${platform} has no generated SHA-256 checksum`);
    }
    if (!/^https:\/\//.test(artifact.url ?? "")) {
      throw new Error(`${name} ${platform} has no HTTPS artifact URL`);
    }
    if (!ARCH_MARKERS[platform].test(artifact.url)) {
      throw new Error(
        `${name} ${platform} artifact architecture does not match its platform key: ${artifact.url}`,
      );
    }
  }
}

export function validateLockfile() {
  const config = readToml("mise.toml");
  const configTools = Object.keys(config.tools ?? {});
  if (
    configTools.length !== 1 ||
    configTools[0] !== "uv" ||
    config.tools.uv !== "latest"
  ) {
    throw new Error('mise.toml [tools] must contain only uv = "latest"');
  }
  if (config.tool_alias?.uv !== "github:astral-sh/uv") {
    throw new Error("mise.toml must use the audited GitHub uv backend alias");
  }
  if (config.tool_alias?.pnpm !== "github:pnpm/pnpm") {
    throw new Error("mise.toml must use the audited GitHub pnpm backend alias");
  }
  const idiomatic = config.settings?.idiomatic_version_file_enable_tools ?? [];
  if (JSON.stringify(idiomatic) !== JSON.stringify(["node", "pnpm", "rust"])) {
    throw new Error(
      "mise.toml must enable only Node, pnpm, and Rust idiomatic files",
    );
  }
  if (config.settings?.locked === true) {
    throw new Error(
      "Project configuration must not enable mise global strict locked mode",
    );
  }
  if (
    JSON.stringify(config.settings?.lockfile_platforms) !==
    JSON.stringify(SUPPORTED_PLATFORMS)
  ) {
    throw new Error("mise lockfile platform order or coverage drifted");
  }

  const lockSource = read("mise.lock");
  if (!lockSource.startsWith("# @generated")) {
    throw new Error("mise.lock must retain its generated-file marker");
  }
  if (/llvm-tools|^targets\s*=/m.test(lockSource)) {
    throw new Error(
      "mise.lock must not provision llvm-tools or non-host Rust targets",
    );
  }
  const lock = readToml("mise.lock");
  if (lock.tools?.python)
    throw new Error("Python must not be installed by mise");

  for (const name of ["node", "pnpm", "uv"]) {
    const entry = entriesFor(lock, name);
    if (name !== "uv" && entry.version !== EXPECTED[name]) {
      throw new Error(`${name} lock version drifted: ${entry.version}`);
    }
    validateArtifactPlatforms(name, entry);
  }
  if (entriesFor(lock, "node").backend !== "core:node") {
    throw new Error("Node lock backend drifted from core:node");
  }
  const uv = entriesFor(lock, "uv");
  if (uv.backend !== "github:astral-sh/uv") {
    throw new Error("uv lock backend must match the audited repository alias");
  }
  if (!/^\d+\.\d+\.\d+$/.test(uv.version)) {
    throw new Error(
      "uv latest selector must resolve to one exact lock version",
    );
  }
  if (
    !uv["platforms.windows-arm64"].url.includes("uv-aarch64-pc-windows-msvc")
  ) {
    throw new Error(
      "Windows ARM64 uv lock must select the native aarch64 asset",
    );
  }
  const pnpm = entriesFor(lock, "pnpm");
  if (pnpm.backend !== "github:pnpm/pnpm") {
    throw new Error(
      "pnpm lock backend must match the audited repository alias",
    );
  }
  if (!pnpm["platforms.windows-arm64"].url.includes("pnpm-win-arm64.exe")) {
    throw new Error(
      "Windows ARM64 pnpm lock must select the native ARM64 executable",
    );
  }

  const rust = entriesFor(lock, "rust");
  if (rust.backend !== "core:rust") {
    throw new Error("Rust lock backend drifted from core:rust");
  }
  if (rust.version !== EXPECTED.rust) {
    throw new Error(`Rust lock version drifted: ${rust.version}`);
  }
  if (
    rust.options?.profile !== "minimal" ||
    rust.options?.components !== "clippy,rustfmt"
  ) {
    throw new Error("Rust lock must retain minimal + clippy,rustfmt options");
  }
  const rustPlatformKeys = Object.keys(rust).filter((key) =>
    key.startsWith("platforms."),
  );
  if (rustPlatformKeys.length > 0) {
    validateArtifactPlatforms("rust", rust);
  }

  const packageJson = readJson("package.json");
  if (read(".node-version").trim() !== EXPECTED.node) {
    throw new Error(".node-version drifted");
  }
  if (packageJson.packageManager !== `pnpm@${EXPECTED.pnpm}`) {
    throw new Error("packageManager drifted");
  }
  if (!read("rust-toolchain.toml").includes('channel = "1.97.1"')) {
    throw new Error("rust-toolchain.toml drifted");
  }
  if (read(".python-version").trim() !== "3.14.7") {
    throw new Error(".python-version drifted");
  }
  if (!read("uv.lock").includes('requires-python = "==3.14.*"')) {
    throw new Error("uv.lock does not encode the Python 3.14 project contract");
  }

  return {
    ok: true,
    tools: {
      node: EXPECTED.node,
      pnpm: EXPECTED.pnpm,
      rust: EXPECTED.rust,
      uv: uv.version,
      python: "3.14.7",
    },
    artifactPlatforms: SUPPORTED_PLATFORMS,
    rustLockCoverage:
      rustPlatformKeys.length > 0
        ? "artifact-checksums"
        : "exact-version-and-options (core:rust publishes no lockable platform assets)",
  };
}

if (isMain(import.meta.url)) {
  try {
    console.log(JSON.stringify(validateLockfile(), null, 2));
  } catch (error) {
    fail(error);
  }
}
