#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import {
  ROOT,
  fail,
  repositoryPath,
  run,
  usageBoolean,
  usageList,
  usageValue,
} from "./lib.mjs";

const REQUIRED_ICONS = Object.freeze([
  "32x32.png",
  "128x128.png",
  "128x128@2x.png",
  "icon.icns",
  "icon.ico",
  "icon.png",
  "StoreLogo.png",
]);

function test(watch) {
  const filters = usageList("filters");
  for (const filter of filters) {
    if (filter.startsWith("-")) {
      throw new Error(
        "test filters accept file or test-name values only; Vitest options are forbidden",
      );
    }
  }
  run("pnpm", ["exec", "vitest", watch ? "watch" : "run", ...filters]);
}

function visualUpdate() {
  const evidence = usageValue("evidence");
  if (!evidence) throw new Error("A reviewed evidence JSON file is required");
  repositoryPath(evidence);
  run("pnpm", ["test:desktop:visual:update", evidence]);
}

function assetsIcons() {
  const source = usageValue("source") ?? "assets/fyagent.png";
  const absoluteSource = repositoryPath(source);
  if (!fs.statSync(absoluteSource).isFile()) {
    throw new Error(`Icon source is not a regular file: ${source}`);
  }
  const extension = path.extname(source).toLowerCase();
  if (extension !== ".png" && extension !== ".svg") {
    throw new Error("Icon source must be a PNG or SVG file");
  }
  const args = ["tauri", "icon", source, "--output", "src-tauri/icons"];
  if (!usageBoolean("apply")) {
    console.log(
      JSON.stringify(
        { status: "preview", command: ["pnpm", ...args] },
        null,
        2,
      ),
    );
    return;
  }
  run("pnpm", args);
}

function assetsIconsCheck() {
  for (const name of REQUIRED_ICONS) {
    const absolute = path.join(ROOT, "src-tauri", "icons", name);
    if (!fs.statSync(absolute).isFile() || fs.statSync(absolute).size === 0) {
      throw new Error(
        `Missing or empty required icon: src-tauri/icons/${name}`,
      );
    }
  }
  for (const name of REQUIRED_ICONS.filter((name) => name.endsWith(".png"))) {
    const signature = fs
      .readFileSync(path.join(ROOT, "src-tauri", "icons", name))
      .subarray(0, 8)
      .toString("hex");
    if (signature !== "89504e470d0a1a0a") {
      throw new Error(`Invalid PNG signature: src-tauri/icons/${name}`);
    }
  }
  console.log(`Verified ${REQUIRED_ICONS.length} required icon consumers.`);
}

try {
  switch (process.argv[2]) {
    case "test-unit":
      test(false);
      break;
    case "test-watch":
      test(true);
      break;
    case "visual-update":
      visualUpdate();
      break;
    case "assets-icons":
      assetsIcons();
      break;
    case "assets-icons-check":
      assetsIconsCheck();
      break;
    default:
      throw new Error(
        `Unknown frontend task command: ${process.argv[2] ?? ""}`,
      );
  }
} catch (error) {
  fail(error);
}
