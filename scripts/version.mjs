#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repositoryRoot = path.resolve(path.dirname(scriptPath), "..");

const paths = Object.freeze({
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
});

const STABLE_SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const LOCAL_CARGO_PACKAGES = Object.freeze([
  "fyagent",
  "fyagent-installer-actions",
]);
const REQUIRED_WORKSPACE_MEMBERS = Object.freeze([".", "installer-actions"]);
const REQUIRED_PACKAGE_SCRIPTS = Object.freeze({
  "version:get": "node scripts/version.mjs get",
  "version:check": "node scripts/version.mjs check",
  "version:set": "node scripts/version.mjs set",
  "version:bump": "node scripts/version.mjs bump",
});

function fail(message) {
  throw new Error(message);
}

function relativePath(filePath) {
  return path.relative(repositoryRoot, filePath);
}

function readText(filePath) {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch (error) {
    fail(
      "cannot read " +
        relativePath(filePath) +
        ": " +
        (error instanceof Error ? error.message : String(error)),
    );
  }
}

function readJson(filePath) {
  const text = readText(filePath);
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(
      "invalid JSON in " +
        relativePath(filePath) +
        ": " +
        (error instanceof Error ? error.message : String(error)),
    );
  }
}

function splitLines(text) {
  return {
    lines: text.split(/\r?\n/),
    eol: text.includes("\r\n") ? "\r\n" : "\n",
  };
}

function escapeRegExp(value) {
  return value.replace(/[|\\{}()[\]^$+*?.]/g, "\\$&");
}

function findTomlSection(text, sectionName, fileLabel) {
  const { lines, eol } = splitLines(text);
  const wanted = "[" + sectionName + "]";
  const starts = lines
    .map((line, index) => (line.trim() === wanted ? index : -1))
    .filter((index) => index >= 0);
  if (starts.length === 0) {
    fail(fileLabel + " is missing " + wanted);
  }
  if (starts.length !== 1) {
    fail(
      fileLabel +
        " must declare " +
        wanted +
        " exactly once; found " +
        starts.length,
    );
  }
  const start = starts[0];

  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    if (/^\s*\[/.test(lines[index])) {
      end = index;
      break;
    }
  }

  return { lines, eol, start, end };
}

function findTomlAssignments(section, key) {
  const pattern = new RegExp(
    "^\\s*" + escapeRegExp(key) + "\\s*=\\s*(.*?)\\s*(?:#.*)?$",
  );
  const matches = [];

  for (let index = section.start + 1; index < section.end; index += 1) {
    const match = section.lines[index].match(pattern);
    if (match) {
      matches.push({ index, value: match[1].trim() });
    }
  }

  return matches;
}

function readExactlyOneTomlString(section, key, fileLabel) {
  const matches = findTomlAssignments(section, key);
  if (matches.length !== 1) {
    fail(
      fileLabel +
        " " +
        key +
        " must be declared exactly once; found " +
        matches.length,
    );
  }

  const value = matches[0].value.match(/^"([^"]*)"$/);
  if (!value) {
    fail(fileLabel + " " + key + " must be a quoted string literal");
  }
  return value[1];
}

function readExactlyOneTomlStringArray(section, key, fileLabel) {
  const keyPattern = new RegExp(
    "^\\s*" + escapeRegExp(key) + "\\s*=\\s*(.*?)(?:\\s+#.*)?$",
  );
  const starts = [];

  for (let index = section.start + 1; index < section.end; index += 1) {
    const match = section.lines[index].match(keyPattern);
    if (match) {
      starts.push({ index, value: match[1] });
    }
  }

  if (starts.length !== 1) {
    fail(
      fileLabel +
        " " +
        key +
        " must be declared exactly once; found " +
        starts.length,
    );
  }

  let expression = starts[0].value.trim();
  for (
    let index = starts[0].index + 1;
    !expression.includes("]") && index < section.end;
    index += 1
  ) {
    expression += " " + section.lines[index].trim();
  }

  const array = expression.match(/^\[([\s\S]*)\]$/);
  if (!array) {
    fail(fileLabel + " " + key + " must be a literal string array");
  }

  const contents = array[1];
  const members = [...contents.matchAll(/"([^"]*)"/g)].map((match) => match[1]);
  const remaining = contents.replace(/"[^"]*"/g, "").replace(/[\s,]/g, "");
  if (remaining) {
    fail(fileLabel + " " + key + " must contain only quoted string values");
  }
  return members;
}

function readWorkspaceVersion(cargoText) {
  const section = findTomlSection(
    cargoText,
    "workspace.package",
    "src-tauri/Cargo.toml",
  );
  return readExactlyOneTomlString(
    section,
    "version",
    "src-tauri/Cargo.toml [workspace.package]",
  );
}

function replaceWorkspaceVersion(cargoText, nextVersion) {
  const section = findTomlSection(
    cargoText,
    "workspace.package",
    "src-tauri/Cargo.toml",
  );
  const matches = findTomlAssignments(section, "version");
  if (matches.length !== 1 || !/^"[^"]*"$/.test(matches[0]?.value ?? "")) {
    fail(
      "src-tauri/Cargo.toml [workspace.package] must contain exactly one literal version; found " +
        matches.length,
    );
  }

  const index = matches[0].index;
  section.lines[index] = section.lines[index].replace(
    /^(\s*version\s*=\s*)"[^"]+"(\s*(?:#.*)?)$/,
    '$1"' + nextVersion + '"$2',
  );
  return section.lines.join(section.eol);
}

function collectWorkspaceErrors(cargoText) {
  const errors = [];
  let workspace;
  try {
    workspace = findTomlSection(cargoText, "workspace", "src-tauri/Cargo.toml");
  } catch (error) {
    return [error instanceof Error ? error.message : String(error)];
  }

  try {
    const members = readExactlyOneTomlStringArray(
      workspace,
      "members",
      "src-tauri/Cargo.toml [workspace]",
    );
    const sameMembers =
      members.length === REQUIRED_WORKSPACE_MEMBERS.length &&
      members.every(
        (member, index) => member === REQUIRED_WORKSPACE_MEMBERS[index],
      );
    if (!sameMembers) {
      errors.push(
        'src-tauri/Cargo.toml [workspace] members must be [".", "installer-actions"]',
      );
    }
  } catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
  }

  try {
    const resolver = readExactlyOneTomlString(
      workspace,
      "resolver",
      "src-tauri/Cargo.toml [workspace]",
    );
    if (resolver !== "2") {
      errors.push('src-tauri/Cargo.toml [workspace] resolver must be "2"');
    }
  } catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
  }

  return errors;
}

function collectPackageManifestErrors(
  manifestText,
  fileLabel,
  expectedPackageName,
) {
  const errors = [];
  let section;
  try {
    section = findTomlSection(manifestText, "package", fileLabel);
  } catch (error) {
    return [error instanceof Error ? error.message : String(error)];
  }

  try {
    const packageName = readExactlyOneTomlString(
      section,
      "name",
      fileLabel + " [package]",
    );
    if (packageName !== expectedPackageName) {
      errors.push(
        fileLabel +
          " [package] name must be " +
          JSON.stringify(expectedPackageName) +
          "; found " +
          JSON.stringify(packageName),
      );
    }
  } catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
  }

  const inherited = section.lines
    .slice(section.start + 1, section.end)
    .filter((line) =>
      /^\s*version\.workspace\s*=\s*true\s*(?:#.*)?$/.test(line),
    );
  const literalVersions = section.lines
    .slice(section.start + 1, section.end)
    .filter((line) => /^\s*version\s*=/.test(line));
  const workspaceVersionAssignments = section.lines
    .slice(section.start + 1, section.end)
    .filter((line) => /^\s*version\.workspace\s*=/.test(line));

  if (
    inherited.length !== 1 ||
    literalVersions.length !== 0 ||
    workspaceVersionAssignments.length !== 1
  ) {
    errors.push(
      fileLabel +
        " [package] must use exactly one version.workspace = true and no literal version",
    );
  }

  return errors;
}

function parseCargoLockPackages(lockText) {
  const starts = [];
  const marker = /^\[\[package\]\][ \t]*\r?$/gm;
  for (
    let match = marker.exec(lockText);
    match;
    match = marker.exec(lockText)
  ) {
    starts.push(match.index);
  }

  return starts.map((start, index) => {
    const end = starts[index + 1] ?? lockText.length;
    const block = lockText.slice(start, end);
    const names = [...block.matchAll(/^name\s*=\s*"([^"]+)"[ \t]*\r?$/gm)].map(
      (match) => match[1],
    );
    const versions = [
      ...block.matchAll(/^version\s*=\s*"([^"]+)"[ \t]*\r?$/gm),
    ].map((match) => match[1]);
    const sources = [
      ...block.matchAll(/^source\s*=\s*"([^"]+)"[ \t]*\r?$/gm),
    ].map((match) => match[1]);
    return {
      start,
      end,
      block,
      name: names[0] ?? null,
      nameCount: names.length,
      version: versions[0] ?? null,
      versionCount: versions.length,
      source: sources[0] ?? null,
      sourceCount: sources.length,
    };
  });
}

function inspectLocalLockPackages(
  lockText,
  version,
  { allowVersionDrift = false } = {},
) {
  const byName = new Map(
    LOCAL_CARGO_PACKAGES.map((packageName) => [packageName, []]),
  );
  for (const entry of parseCargoLockPackages(lockText)) {
    if (entry.name && byName.has(entry.name)) {
      byName.get(entry.name).push(entry);
    }
  }

  const errors = [];
  for (const packageName of LOCAL_CARGO_PACKAGES) {
    const entries = byName.get(packageName) ?? [];
    if (entries.length === 0) {
      errors.push(
        "src-tauri/Cargo.lock is missing local package " + packageName,
      );
      continue;
    }
    if (entries.length !== 1) {
      errors.push(
        "src-tauri/Cargo.lock contains duplicate local package " + packageName,
      );
      continue;
    }

    const entry = entries[0];
    if (entry.nameCount !== 1) {
      errors.push(
        "src-tauri/Cargo.lock package " +
          packageName +
          " must contain exactly one name",
      );
    }
    if (entry.sourceCount !== 0) {
      errors.push(
        "src-tauri/Cargo.lock " +
          packageName +
          " must be a local workspace package without source",
      );
    }
    if (entry.versionCount !== 1) {
      errors.push(
        "src-tauri/Cargo.lock package " +
          packageName +
          " must contain exactly one version",
      );
    } else if (!allowVersionDrift && entry.version !== version) {
      errors.push(
        "src-tauri/Cargo.lock " +
          packageName +
          "=" +
          JSON.stringify(entry.version) +
          " does not match " +
          version,
      );
    }
  }

  return { byName, errors };
}

function replaceLocalLockVersions(lockText, nextVersion) {
  const currentVersion = readWorkspaceVersion(readText(paths.cargoManifest));
  const inspection = inspectLocalLockPackages(lockText, currentVersion, {
    allowVersionDrift: true,
  });
  if (inspection.errors.length > 0) {
    fail(
      "cannot update local Cargo.lock package entries:\n  - " +
        inspection.errors.join("\n  - "),
    );
  }

  const entries = parseCargoLockPackages(lockText);
  let output = "";
  let cursor = 0;
  for (const entry of entries) {
    output += lockText.slice(cursor, entry.start);
    let block = entry.block;
    if (entry.name && LOCAL_CARGO_PACKAGES.includes(entry.name)) {
      block = block.replace(
        /^(\s*version\s*=\s*)"[^"]+"([ \t]*)(\r?)$/m,
        '$1"' + nextVersion + '"$2$3',
      );
    }
    output += block;
    cursor = entry.end;
  }
  output += lockText.slice(cursor);
  return output;
}

function validateVersion(version) {
  const match = version.match(STABLE_SEMVER);
  if (!match) {
    fail(
      "FyAgent release version must be a stable SemVer X.Y.Z without v-prefix, prerelease, or build metadata; received " +
        JSON.stringify(version),
    );
  }

  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);
  if (major > 255 || minor > 255 || patch > 65535) {
    fail(
      "version " +
        version +
        " exceeds Windows Installer ProductVersion limits (major/minor <= 255, patch <= 65535)",
    );
  }
  return { major, minor, patch };
}

function collectPackageJsonErrors(packageJson) {
  const errors = [];
  if (Object.prototype.hasOwnProperty.call(packageJson, "version")) {
    errors.push(
      "package.json must not declare the FyAgent application version",
    );
  }
  if (packageJson.private !== true) {
    errors.push('package.json must contain "private": true');
  }

  const scripts = packageJson.scripts;
  if (!scripts || typeof scripts !== "object" || Array.isArray(scripts)) {
    errors.push("package.json scripts must be an object");
    return errors;
  }

  for (const [name, expected] of Object.entries(REQUIRED_PACKAGE_SCRIPTS)) {
    if (scripts[name] !== expected) {
      errors.push(
        "package.json scripts." + name + " must be " + JSON.stringify(expected),
      );
    }
  }
  return errors;
}

function inspectContract({ tag, allowLocalLockVersionDrift = false } = {}) {
  const cargoText = readText(paths.cargoManifest);
  const cargoLockText = readText(paths.cargoLock);
  const packageJson = readJson(paths.packageJson);
  const tauriConfig = readJson(paths.tauriConfig);
  const version = readWorkspaceVersion(cargoText);
  validateVersion(version);

  const errors = [
    ...collectWorkspaceErrors(cargoText),
    ...collectPackageManifestErrors(
      cargoText,
      "src-tauri/Cargo.toml",
      "fyagent",
    ),
    ...collectPackageJsonErrors(packageJson),
  ];

  if (!fs.existsSync(paths.installerActionsManifest)) {
    errors.push(
      "src-tauri/installer-actions/Cargo.toml is required for the FyAgent workspace",
    );
  } else {
    errors.push(
      ...collectPackageManifestErrors(
        readText(paths.installerActionsManifest),
        "src-tauri/installer-actions/Cargo.toml",
        "fyagent-installer-actions",
      ),
    );
  }

  if (Object.prototype.hasOwnProperty.call(tauriConfig, "version")) {
    errors.push(
      "src-tauri/tauri.conf.json must omit version so Tauri inherits the Cargo package version",
    );
  }

  errors.push(
    ...inspectLocalLockPackages(cargoLockText, version, {
      allowVersionDrift: allowLocalLockVersionDrift,
    }).errors,
  );

  if (tag !== undefined && tag !== "v" + version) {
    errors.push(
      "release tag must be v" + version + "; received " + JSON.stringify(tag),
    );
  }

  return { version, errors };
}

function checkContract(options = {}) {
  const result = inspectContract(options);
  if (result.errors.length > 0) {
    fail("version contract failed:\n  - " + result.errors.join("\n  - "));
  }
  return result.version;
}

function snapshotChanges(changes) {
  return new Map(changes.map(({ filePath }) => [filePath, readText(filePath)]));
}

function replaceFileAtomically(filePath, content) {
  const directory = path.dirname(filePath);
  const temporaryPath = path.join(
    directory,
    "." +
      path.basename(filePath) +
      ".fyagent-version-" +
      process.pid +
      "-" +
      randomUUID() +
      ".tmp",
  );
  const mode = fs.statSync(filePath).mode;
  let descriptor;

  try {
    descriptor = fs.openSync(temporaryPath, "wx", mode);
    fs.writeFileSync(descriptor, content, "utf8");
    fs.fsyncSync(descriptor);
    fs.closeSync(descriptor);
    descriptor = undefined;
    fs.renameSync(temporaryPath, filePath);
  } finally {
    if (descriptor !== undefined) {
      try {
        fs.closeSync(descriptor);
      } catch {
        // Preserve the replacement failure; the unique temporary path is still
        // cleaned below and the original target has not been renamed.
      }
    }
    try {
      fs.unlinkSync(temporaryPath);
    } catch (error) {
      if (
        !(error instanceof Error && "code" in error && error.code === "ENOENT")
      ) {
        throw error;
      }
    }
  }
}

function restoreFiles(originals, filePaths) {
  const errors = [];
  for (const filePath of filePaths) {
    try {
      replaceFileAtomically(filePath, originals.get(filePath));
    } catch (error) {
      errors.push(
        relativePath(filePath) +
          ": " +
          (error instanceof Error ? error.message : String(error)),
      );
    }
  }
  return errors;
}

function writeWithRollback(changes, originals) {
  const touched = [];
  try {
    for (const { filePath, content } of changes) {
      replaceFileAtomically(filePath, content);
      touched.push(filePath);
    }
  } catch (error) {
    const rollbackErrors = restoreFiles(originals, [...touched].reverse());
    const rollbackSuffix =
      rollbackErrors.length === 0
        ? "rollback succeeded"
        : "rollback also failed: " + rollbackErrors.join("; ");
    fail(
      "version update failed; " +
        rollbackSuffix +
        ": " +
        (error instanceof Error ? error.message : String(error)),
    );
  }
}

function setVersion(nextVersion, { apply = false } = {}) {
  validateVersion(nextVersion);

  const cargoText = readText(paths.cargoManifest);
  const lockText = readText(paths.cargoLock);
  const currentVersion = readWorkspaceVersion(cargoText);
  validateVersion(currentVersion);

  // A version-set invocation may repair only local lockfile version drift. Every
  // other contract violation is rejected before either writable file is touched.
  const preflight = inspectContract({ allowLocalLockVersionDrift: true });
  if (preflight.errors.length > 0) {
    fail(
      "cannot update an invalid version contract:\n  - " +
        preflight.errors.join("\n  - "),
    );
  }

  const nextCargoText = replaceWorkspaceVersion(cargoText, nextVersion);
  const nextLockText = replaceLocalLockVersions(lockText, nextVersion);
  const changes = [];
  if (nextCargoText !== cargoText) {
    changes.push({ filePath: paths.cargoManifest, content: nextCargoText });
  }
  if (nextLockText !== lockText) {
    changes.push({ filePath: paths.cargoLock, content: nextLockText });
  }

  if (!apply) {
    console.log(currentVersion + " -> " + nextVersion);
    for (const { filePath } of changes) {
      console.log("would update " + relativePath(filePath));
    }
    if (changes.length === 0) {
      console.log("no files would change");
    }
    return;
  }

  const originals = snapshotChanges(changes);
  writeWithRollback(changes, originals);

  try {
    checkContract();
  } catch (error) {
    const rollbackErrors = restoreFiles(
      originals,
      [...changes].map(({ filePath }) => filePath).reverse(),
    );
    const rollbackSuffix =
      rollbackErrors.length === 0
        ? "rollback succeeded"
        : "rollback also failed: " + rollbackErrors.join("; ");
    fail(
      "version update failed contract verification; " +
        rollbackSuffix +
        ": " +
        (error instanceof Error ? error.message : String(error)),
    );
  }

  console.log(currentVersion + " -> " + nextVersion);
  for (const { filePath } of changes) {
    console.log("updated " + relativePath(filePath));
  }
  if (changes.length === 0) {
    console.log("version already matched; no files changed");
  }
}

function bumpVersion(currentVersion, kind) {
  const { major, minor, patch } = validateVersion(currentVersion);
  switch (kind) {
    case "patch":
      return major + "." + minor + "." + (patch + 1);
    case "minor":
      return major + "." + (minor + 1) + ".0";
    case "major":
      return major + 1 + ".0.0";
    default:
      fail(
        "bump kind must be patch, minor, or major; received " +
          JSON.stringify(kind),
      );
  }
}

function parseOptions(args) {
  const options = {
    apply: false,
    dryRun: false,
    tag: undefined,
    positional: [],
  };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    // pnpm forwards its argument separator to lifecycle scripts. It does not
    // carry version-command meaning, so accept and discard it before parsing
    // the command's own options.
    if (arg === "--") {
      continue;
    }
    if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "--apply") {
      options.apply = true;
    } else if (arg === "--tag") {
      const value = args[index + 1];
      if (!value) {
        fail("--tag requires a value");
      }
      options.tag = value;
      index += 1;
    } else if (arg.startsWith("--tag=")) {
      options.tag = arg.slice("--tag=".length);
    } else if (arg.startsWith("--")) {
      fail("unknown option " + arg);
    } else {
      options.positional.push(arg);
    }
  }
  if (options.apply && options.dryRun) {
    fail("--apply and --dry-run are mutually exclusive");
  }
  return options;
}

function printUsage() {
  console.log(
    [
      "Usage:",
      "  node scripts/version.mjs get",
      "  node scripts/version.mjs check [--tag vX.Y.Z]",
      "  node scripts/version.mjs set X.Y.Z [--apply | --dry-run]",
      "  node scripts/version.mjs bump patch|minor|major [--apply | --dry-run]",
      "",
      "The canonical FyAgent application version is src-tauri/Cargo.toml",
      "[workspace.package].version. The script updates only the canonical value",
      "and local Cargo.lock package entries; dependency, toolchain, schema,",
      "protocol, and historical documentation versions are outside its scope.",
      "set and bump preview by default; pass --apply to write the two files.",
    ].join("\n"),
  );
}

function main() {
  const [command = "help", ...rest] = process.argv.slice(2);
  const options = parseOptions(rest);

  switch (command) {
    case "get": {
      if (
        options.positional.length !== 0 ||
        options.tag ||
        options.dryRun ||
        options.apply
      ) {
        fail("get does not accept arguments");
      }
      const version = readWorkspaceVersion(readText(paths.cargoManifest));
      validateVersion(version);
      console.log(version);
      break;
    }
    case "check": {
      if (options.positional.length !== 0 || options.dryRun || options.apply) {
        fail("check accepts only --tag vX.Y.Z");
      }
      const version = checkContract({ tag: options.tag });
      console.log("FyAgent version contract OK: " + version);
      break;
    }
    case "set": {
      if (options.positional.length !== 1 || options.tag) {
        fail(
          "set requires exactly one X.Y.Z argument and optionally --apply or --dry-run",
        );
      }
      setVersion(options.positional[0], { apply: options.apply });
      break;
    }
    case "bump": {
      if (options.positional.length !== 1 || options.tag) {
        fail(
          "bump requires patch, minor, or major and optionally --apply or --dry-run",
        );
      }
      const current = checkContract();
      const next = bumpVersion(current, options.positional[0]);
      setVersion(next, { apply: options.apply });
      break;
    }
    case "help":
    case "--help":
    case "-h":
      printUsage();
      break;
    default:
      fail("unknown command " + JSON.stringify(command));
  }
}

try {
  main();
} catch (error) {
  console.error(
    "[fyagent-version] " +
      (error instanceof Error ? error.message : String(error)),
  );
  process.exitCode = 1;
}
