#!/usr/bin/env node

import fs from "node:fs";
import process from "node:process";
import {
  ROOT,
  SUPPORTED_PLATFORMS,
  assertSimplePackageNames,
  assertStableSemver,
  capture,
  fail,
  printPlan,
  read,
  readJson,
  run,
  usageBoolean,
  usageList,
  usageValue,
  writeFilesAtomically,
} from "./lib.mjs";

const platformArgument = SUPPORTED_PLATFORMS.join(",");

function withFileRollback(relativePaths, operation) {
  const originals = new Map(
    relativePaths.map((relativePath) => [
      relativePath,
      fs.existsSync(`${ROOT}/${relativePath}`)
        ? fs.readFileSync(`${ROOT}/${relativePath}`)
        : null,
    ]),
  );
  try {
    operation();
  } catch (error) {
    for (const [relativePath, original] of originals) {
      const absolute = `${ROOT}/${relativePath}`;
      if (original === null) fs.rmSync(absolute, { force: true });
      else fs.writeFileSync(absolute, original);
    }
    throw error;
  }
}

function dependencyUpdate(ecosystem) {
  const all = usageBoolean("all");
  const apply = usageBoolean("apply");
  const argumentName = ecosystem === "frontend" ? "packages" : "crates";
  const values = usageList(argumentName);
  if (all === values.length > 0) {
    throw new Error(
      `Choose exactly one of named ${argumentName} or --all for a dependency update`,
    );
  }
  if (values.length > 0)
    assertSimplePackageNames(values, argumentName.slice(0, -1));

  const command = ecosystem === "frontend" ? "pnpm" : "cargo";
  const args =
    ecosystem === "frontend"
      ? ["update", ...(all ? ["--latest"] : [...values, "--latest"])]
      : [
          "update",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          ...(all ? [] : values.flatMap((name) => ["--package", name])),
        ];
  if (!apply) {
    printPlan(`update ${ecosystem} dependencies`, command, args);
    return;
  }
  run(command, args);
}

function toolchainUpdate(tool) {
  const version = assertStableSemver(usageValue("version"), `${tool} version`);
  if (!usageBoolean("apply")) {
    printPlan(`update ${tool} toolchain`, "mise", [
      "lock",
      "--platform",
      platformArgument,
    ]);
    console.log(`Would set ${tool} to ${version}.`);
    return;
  }

  const changedFiles = ["mise.lock"];
  let change;
  if (tool === "node") {
    changedFiles.push(".node-version");
    change = [".node-version", `${version}\n`];
  } else if (tool === "rust") {
    changedFiles.push("rust-toolchain.toml");
    change = [
      "rust-toolchain.toml",
      `[toolchain]\nchannel = "${version}"\ncomponents = ["rustfmt", "clippy"]\nprofile = "minimal"\n`,
    ];
  } else if (tool === "pnpm") {
    changedFiles.push("package.json");
    const packageJson = readJson("package.json");
    packageJson.packageManager = `pnpm@${version}`;
    change = ["package.json", `${JSON.stringify(packageJson, null, 2)}\n`];
  } else {
    throw new Error(`Unsupported toolchain update: ${tool}`);
  }

  withFileRollback(changedFiles, () => {
    writeFilesAtomically([change]);
    run("mise", ["lock", "--platform", platformArgument]);
    run("node", ["scripts/tasks/lockfile-check.mjs"]);
  });
}

function versionCommand(command) {
  const apply = usageBoolean("apply");
  if (command === "check") {
    const tag = usageValue("tag");
    run("pnpm", ["run", "version:check", ...(tag ? ["--", "--tag", tag] : [])]);
    return;
  }
  const value = usageValue(command === "set" ? "version" : "level");
  if (!value) throw new Error(`version:${command} requires a value`);
  if (command === "set") assertStableSemver(value, "product version");
  if (command === "bump" && !["patch", "minor", "major"].includes(value)) {
    throw new Error("Version bump level must be patch, minor, or major");
  }
  run("pnpm", [
    "run",
    `version:${command}`,
    "--",
    value,
    ...(apply ? ["--apply"] : []),
  ]);
}

try {
  switch (process.argv[2]) {
    case "deps-outdated-frontend":
      run("pnpm", ["outdated"], { allowFailure: true });
      break;
    case "deps-outdated-rust":
      run("cargo", [
        "update",
        "--dry-run",
        "--manifest-path",
        "src-tauri/Cargo.toml",
      ]);
      break;
    case "deps-outdated-python":
      run("uv", ["tree", "--outdated", "--locked"]);
      break;
    case "deps-update-frontend":
      dependencyUpdate("frontend");
      break;
    case "deps-update-rust":
      dependencyUpdate("rust");
      break;
    case "toolchain-outdated": {
      const current = {
        node: read(".node-version").trim(),
        pnpm: readJson("package.json").packageManager.replace(/^pnpm@/, ""),
        rust: read("rust-toolchain.toml").match(/channel\s*=\s*"([^"]+)"/)?.[1],
        uv: capture("mise", ["current", "uv"]),
      };
      const candidates = {
        node: capture("mise", ["latest", "node"]),
        pnpm: capture("mise", ["latest", "pnpm"]),
        rust: capture("mise", ["latest", "rust"]),
        uv: JSON.parse(
          capture("mise", ["lock", "uv", "--bump", "--dry-run", "--json"]),
        ),
      };
      console.log(JSON.stringify({ current, candidates }, null, 2));
      break;
    }
    case "toolchain-update-node":
      toolchainUpdate("node");
      break;
    case "toolchain-update-rust":
      toolchainUpdate("rust");
      break;
    case "toolchain-update-pnpm":
      toolchainUpdate("pnpm");
      break;
    case "toolchain-update-uv":
      if (!usageBoolean("apply")) {
        run("mise", ["lock", "uv", "--bump", "--dry-run", "--json"]);
      } else {
        withFileRollback(["mise.lock"], () => {
          run("mise", ["lock", "uv", "--bump", "--platform", platformArgument]);
          run("node", ["scripts/tasks/lockfile-check.mjs"]);
        });
      }
      break;
    case "toolchain-lock":
      if (!usageBoolean("apply")) {
        run("mise", ["lock", "--dry-run", "--platform", platformArgument]);
        run("node", ["scripts/tasks/lockfile-check.mjs"]);
      } else {
        withFileRollback(["mise.lock"], () => {
          run("mise", ["lock", "--platform", platformArgument]);
          run("node", ["scripts/tasks/lockfile-check.mjs"]);
        });
      }
      break;
    case "version-check":
      versionCommand("check");
      break;
    case "version-set":
      versionCommand("set");
      break;
    case "version-bump":
      versionCommand("bump");
      break;
    default:
      throw new Error(
        `Unknown maintenance task command: ${process.argv[2] ?? ""}`,
      );
  }
} catch (error) {
  fail(error);
}
