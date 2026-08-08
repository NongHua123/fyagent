#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repositoryRoot = path.resolve(path.dirname(scriptPath), "..");

const paths = {
  packageJson: path.join(repositoryRoot, "package.json"),
  tauriConfig: path.join(repositoryRoot, "src-tauri", "tauri.conf.json"),
  cargoManifest: path.join(repositoryRoot, "src-tauri", "Cargo.toml"),
  cargoLock: path.join(repositoryRoot, "src-tauri", "Cargo.lock"),
  installerActionsManifest: path.join(
    repositoryRoot,
    "src-tauri",
    "installer-actions",
    "Cargo.toml",
  ),
};

const STABLE_SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const LOCAL_CARGO_PACKAGES = ["fyagent", "fyagent-installer-actions"];

function fail(message) {
  throw new Error(message);
}

function readText(filePath) {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch (error) {
    fail(`cannot read ${path.relative(repositoryRoot, filePath)}: ${error.message}`);
  }
}

function readJson(filePath) {
  const text = readText(filePath);
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(`invalid JSON in ${path.relative(repositoryRoot, filePath)}: ${error.message}`);
  }
}

function splitLines(text) {
  const eol = text.includes("\r\n") ? "\r\n" : "\n";
  return { lines: text.split(/\r?\n/), eol };
}

function findTomlSection(text, sectionName, fileLabel) {
  const { lines, eol } = splitLines(text);
  const wanted = `[${sectionName}]`;
  const start = lines.findIndex((line) => line.trim() === wanted);
  if (start < 0) {
    fail(`${fileLabel} is missing ${wanted}`);
  }

  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    if (/^\s*\[\[?.+\]\]?\]\s*$/.test(lines[index])) {
      end = index;
      break;
    }
  }

  return { lines, eol, start, end };
}

function readWorkspaceVersion(cargoText) {
  const section = findTomlSection(
    cargoText,
    "workspace.package",
    "src-tauri/Cargo.toml",
  );
  const matches = [];
  for (let index = section.start + 1; index < section.end; index += 1) {
    const match = section.lines[index].match(/^\s*version\s*=\s*"([^"]+)"\s*(?:#.*)?$/);
    if (match) matches.push({ index, version: match[1] });
  }
  if (matches.length !== 1) {
    fail(
      `src-tauri/Cargo.toml [workspace.package] must contain exactly one literal version; found ${matches.length}`,
    );
  }
  return matches[0].version;
}

function replaceWorkspaceVersion(cargoText, nextVersion) {
  const section = findTomlSection(
    cargoText,
    "workspace.package",
    "src-tauri/Cargo.toml",
  );
  const matches = [];
  for (let index = section.start + 1; index < section.end; index += 1) {
    if (/^\s*version\s*=\s*"[^"]+"\s*(?:#.*)?$/.test(section.lines[index])) {
      matches.push(index);
    }
  }
  if (matches.length !== 1) {
    fail(
      `src-tauri/Cargo.toml [workspace.package] must contain exactly one literal version; found ${matches.length}`,
    );
  }

  const index = matches[0];
  section.lines[index] = section.lines[index].replace(
    /^(\s*version\s*=\s*)"[^"]+"(\s*(?:#.*)?)$/,
    `$1"${nextVersion}"$2`,
  );
  return section.lines.join(section.eol);
}

function sectionHasWorkspaceVersion(manifestText, fileLabel) {
  const section = findTomlSection(manifestText, "package", fileLabel);
  const matching = section.lines
    .slice(section.start + 1, section.end)
    .filter((line) => /^\s*version\.workspace\s*=\s*true\s*(?:#.*)?$/.test(line));
  return matching.length === 1;
}

function parseCargoLockPackages(lockText) {
  const starts = [];
  const marker = /^\[\[package\]\]\s*$/gm;
  for (let match = marker.exec(lockText); match; match = marker.exec(lockText)) {
    starts.push(match.index);
  }

  return starts.map((start, index) => {
    const end = starts[index + 1] ?? lockText.length;
    const block = lockText.slice(start, end);
    const name = block.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1] ?? null;
    const version = block.match(/^version\s*=\s*"([^"]+)"[ \t]*$/m)?.[1] ?? null;
    return { start, end, block, name, version };
  });
}

function localLockVersions(lockText) {
  const result = new Map();
  for (const entry of parseCargoLockPackages(lockText)) {
    if (!entry.name || !LOCAL_CARGO_PACKAGES.includes(entry.name)) continue;
    if (result.has(entry.name)) {
      fail(`src-tauri/Cargo.lock contains duplicate local package ${entry.name}`);
    }
    result.set(entry.name, entry.version ?? "");
  }
  return result;
}

function replaceLocalLockVersions(lockText, nextVersion) {
  const entries = parseCargoLockPackages(lockText);
  const found = new Set();
  let output = "";
  let cursor = 0;

  for (const entry of entries) {
    output += lockText.slice(cursor, entry.start);
    let block = entry.block;
    if (entry.name && LOCAL_CARGO_PACKAGES.includes(entry.name)) {
      if (found.has(entry.name)) {
        fail(`src-tauri/Cargo.lock contains duplicate local package ${entry.name}`);
      }
      found.add(entry.name);
      const versionMatches = block.match(/^version\s*=\s*"[^"]+"[ \t]*$/gm) ?? [];
      if (versionMatches.length !== 1) {
        fail(
          `src-tauri/Cargo.lock package ${entry.name} must contain exactly one version; found ${versionMatches.length}`,
        );
      }
      block = block.replace(
        /^version\s*=\s*"[^"]+"[ \t]*$/m,
        `version = "${nextVersion}"`,
      );
    }
    output += block;
    cursor = entry.end;
  }
  output += lockText.slice(cursor);

  if (!found.has("fyagent")) {
    fail("src-tauri/Cargo.lock does not contain the local fyagent package");
  }
  if (fs.existsSync(paths.installerActionsManifest) && !found.has("fyagent-installer-actions")) {
    fail(
      "src-tauri/Cargo.lock does not contain fyagent-installer-actions even though its manifest exists",
    );
  }

  return output;
}

function validateVersion(version) {
  const match = version.match(STABLE_SEMVER);
  if (!match) {
    fail(
      `FyAgent release version must be a stable SemVer X.Y.Z without v-prefix, prerelease, or build metadata; received ${JSON.stringify(version)}`,
    );
  }

  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);
  if (major > 255 || minor > 255 || patch > 65535) {
    fail(
      `version ${version} exceeds Windows Installer ProductVersion limits (major/minor <= 255, patch <= 65535)`,
    );
  }
  return { major, minor, patch };
}

function inspectContract({ tag } = {}) {
  const cargoText = readText(paths.cargoManifest);
  const cargoLockText = readText(paths.cargoLock);
  const packageJson = readJson(paths.packageJson);
  const tauriConfig = readJson(paths.tauriConfig);
  const version = readWorkspaceVersion(cargoText);
  validateVersion(version);

  const errors = [];
  if (!sectionHasWorkspaceVersion(cargoText, "src-tauri/Cargo.toml")) {
    errors.push('src-tauri/Cargo.toml [package] must use `version.workspace = true`');
  }

  if (fs.existsSync(paths.installerActionsManifest)) {
    const installerManifest = readText(paths.installerActionsManifest);
    if (!sectionHasWorkspaceVersion(
      installerManifest,
      "src-tauri/installer-actions/Cargo.toml",
    )) {
      errors.push(
        'src-tauri/installer-actions/Cargo.toml [package] must use `version.workspace = true`',
      );
    }
  }

  if (Object.prototype.hasOwnProperty.call(packageJson, "version")) {
    errors.push("package.json must not declare the FyAgent application version");
  }
  if (packageJson.private !== true) {
    errors.push('package.json must contain `"private": true`');
  }

  if (Object.prototype.hasOwnProperty.call(tauriConfig, "version")) {
    errors.push(
      "src-tauri/tauri.conf.json must omit version so Tauri inherits the Cargo package version",
    );
  }

  const lockVersions = localLockVersions(cargoLockText);
  if (!lockVersions.has("fyagent")) {
    errors.push("src-tauri/Cargo.lock is missing local package fyagent");
  }
  for (const [name, lockVersion] of lockVersions) {
    if (lockVersion !== version) {
      errors.push(
        `src-tauri/Cargo.lock ${name}=${JSON.stringify(lockVersion)} does not match ${version}`,
      );
    }
  }
  if (fs.existsSync(paths.installerActionsManifest) && !lockVersions.has("fyagent-installer-actions")) {
    errors.push("src-tauri/Cargo.lock is missing local package fyagent-installer-actions");
  }

  if (tag !== undefined && tag !== `v${version}`) {
    errors.push(`release tag must be v${version}; received ${JSON.stringify(tag)}`);
  }

  return { version, errors };
}

function checkContract(options = {}) {
  const result = inspectContract(options);
  if (result.errors.length > 0) {
    fail(`version contract failed:\n  - ${result.errors.join("\n  - ")}`);
  }
  return result.version;
}

function writeWithRollback(changes) {
  const originals = new Map(changes.map(({ filePath }) => [filePath, readText(filePath)]));
  const written = [];
  try {
    for (const { filePath, content } of changes) {
      fs.writeFileSync(filePath, content, "utf8");
      written.push(filePath);
    }
  } catch (error) {
    for (const filePath of written.reverse()) {
      try {
        fs.writeFileSync(filePath, originals.get(filePath), "utf8");
      } catch {
        // The original write error remains primary; report rollback failure below.
      }
    }
    fail(`version update failed and rollback was attempted: ${error.message}`);
  }
}

function setVersion(nextVersion, { dryRun = false } = {}) {
  validateVersion(nextVersion);

  const cargoText = readText(paths.cargoManifest);
  const lockText = readText(paths.cargoLock);
  const currentVersion = readWorkspaceVersion(cargoText);
  validateVersion(currentVersion);

  // Validate the surrounding contract before editing so the script never
  // silently repairs or overwrites an unexpected project layout.
  const preflight = inspectContract();
  const structuralErrors = preflight.errors.filter(
    (error) => !error.startsWith("src-tauri/Cargo.lock "),
  );
  if (structuralErrors.length > 0) {
    fail(`cannot update an invalid version contract:\n  - ${structuralErrors.join("\n  - ")}`);
  }

  const nextCargoText = replaceWorkspaceVersion(cargoText, nextVersion);
  const nextLockText = replaceLocalLockVersions(lockText, nextVersion);
  const changedFiles = [];
  if (nextCargoText !== cargoText) changedFiles.push("src-tauri/Cargo.toml");
  if (nextLockText !== lockText) changedFiles.push("src-tauri/Cargo.lock");

  if (dryRun) {
    console.log(`${currentVersion} -> ${nextVersion}`);
    for (const file of changedFiles) console.log(`would update ${file}`);
    if (changedFiles.length === 0) console.log("no files would change");
    return;
  }

  writeWithRollback([
    { filePath: paths.cargoManifest, content: nextCargoText },
    { filePath: paths.cargoLock, content: nextLockText },
  ]);

  try {
    checkContract();
  } catch (error) {
    writeWithRollback([
      { filePath: paths.cargoManifest, content: cargoText },
      { filePath: paths.cargoLock, content: lockText },
    ]);
    throw error;
  }

  console.log(`${currentVersion} -> ${nextVersion}`);
  for (const file of changedFiles) console.log(`updated ${file}`);
  if (changedFiles.length === 0) console.log("version already matched; no files changed");
}

function bumpVersion(currentVersion, kind) {
  const { major, minor, patch } = validateVersion(currentVersion);
  switch (kind) {
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "major":
      return `${major + 1}.0.0`;
    default:
      fail(`bump kind must be patch, minor, or major; received ${JSON.stringify(kind)}`);
  }
}

function parseOptions(args) {
  const options = { dryRun: false, tag: undefined, positional: [] };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "--tag") {
      const value = args[index + 1];
      if (!value) fail("--tag requires a value");
      options.tag = value;
      index += 1;
    } else if (arg.startsWith("--tag=")) {
      options.tag = arg.slice("--tag=".length);
    } else if (arg.startsWith("--")) {
      fail(`unknown option ${arg}`);
    } else {
      options.positional.push(arg);
    }
  }
  return options;
}

function printUsage() {
  console.log(`Usage:
  node scripts/version.mjs get
  node scripts/version.mjs check [--tag vX.Y.Z]
  node scripts/version.mjs set X.Y.Z [--dry-run]
  node scripts/version.mjs bump patch|minor|major [--dry-run]

The canonical FyAgent application version is src-tauri/Cargo.toml
[workspace.package].version. The script updates only the canonical value and
local Cargo.lock package entries; dependency, toolchain, schema, protocol, and
historical documentation versions are outside its scope.`);
}

function main() {
  const [command = "help", ...rest] = process.argv.slice(2);
  const options = parseOptions(rest);

  switch (command) {
    case "get": {
      if (options.positional.length !== 0 || options.tag || options.dryRun) {
        fail("get does not accept arguments");
      }
      const version = readWorkspaceVersion(readText(paths.cargoManifest));
      validateVersion(version);
      console.log(version);
      break;
    }
    case "check": {
      if (options.positional.length !== 0 || options.dryRun) {
        fail("check accepts only --tag vX.Y.Z");
      }
      const version = checkContract({ tag: options.tag });
      console.log(`FyAgent version contract OK: ${version}`);
      break;
    }
    case "set": {
      if (options.positional.length !== 1 || options.tag) {
        fail("set requires exactly one X.Y.Z argument and optionally --dry-run");
      }
      setVersion(options.positional[0], { dryRun: options.dryRun });
      break;
    }
    case "bump": {
      if (options.positional.length !== 1 || options.tag) {
        fail("bump requires patch, minor, or major and optionally --dry-run");
      }
      const current = checkContract();
      const next = bumpVersion(current, options.positional[0]);
      setVersion(next, { dryRun: options.dryRun });
      break;
    }
    case "help":
    case "--help":
    case "-h":
      printUsage();
      break;
    default:
      fail(`unknown command ${JSON.stringify(command)}`);
  }
}

try {
  main();
} catch (error) {
  console.error(`[fyagent-version] ${error.message}`);
  process.exitCode = 1;
}
