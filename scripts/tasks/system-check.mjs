#!/usr/bin/env node

import process from "node:process";
import { run, usageBoolean } from "./lib.mjs";

export const REQUIREMENTS = Object.freeze({
  linux: {
    commands: [
      ["git", ["--version"], "Install Git with the host package manager."],
      [
        "cc",
        ["--version"],
        "Install the distribution's C/C++ build toolchain.",
      ],
      ["make", ["--version"], "Install the distribution's build toolchain."],
      ["pkg-config", ["--version"], "Install pkg-config."],
    ],
    pkgConfig: [
      ["webkit2gtk-4.1", "Install the WebKitGTK 4.1 development package."],
      [
        "javascriptcoregtk-4.1",
        "Install the JavaScriptCoreGTK 4.1 development package.",
      ],
      ["gtk+-3.0", "Install the GTK 3 development package."],
      ["librsvg-2.0", "Install the librsvg development package."],
      ["openssl", "Install the OpenSSL development package."],
      [
        "ayatana-appindicator3-0.1",
        "Install the Ayatana AppIndicator 3 development package.",
      ],
    ],
  },
  darwin: {
    commands: [
      ["git", ["--version"], "Install the Xcode command-line tools."],
      ["xcode-select", ["-p"], "Run xcode-select --install interactively."],
      ["xcrun", ["--find", "clang"], "Install the Xcode command-line tools."],
    ],
    pkgConfig: [],
  },
  win32: {
    commands: [
      ["git", ["--version"], "Install Git for Windows."],
      [
        "where.exe",
        ["cl.exe"],
        "Open a Visual Studio 2022 Developer shell with the Desktop C++ workload.",
      ],
      [
        "reg.exe",
        [
          "query",
          "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients",
          "/s",
          "/f",
          "WebView2 Runtime",
        ],
        "Install the Microsoft Edge WebView2 Evergreen Runtime.",
      ],
    ],
    pkgConfig: [],
  },
});

function inspect(platform) {
  const requirements = REQUIREMENTS[platform];
  if (!requirements) {
    return {
      ok: false,
      platform,
      checks: [
        {
          name: "supported-host",
          ok: false,
          hint: `Unsupported host platform: ${platform}`,
        },
      ],
    };
  }
  const checks = [];
  for (const [command, args, hint] of requirements.commands) {
    const result = probe(command, args);
    checks.push({
      name: `${command} ${args.join(" ")}`,
      ok: result.status === 0,
      hint: result.status === 0 ? undefined : hint,
    });
  }
  for (const [module, hint] of requirements.pkgConfig) {
    const result = probe("pkg-config", ["--exists", module]);
    checks.push({
      name: `pkg-config ${module}`,
      ok: result.status === 0,
      hint: result.status === 0 ? undefined : hint,
    });
  }
  return { ok: checks.every((check) => check.ok), platform, checks };
}

function probe(command, args) {
  try {
    return run(command, args, { capture: true, allowFailure: true });
  } catch (error) {
    return {
      status: 1,
      stdout: "",
      stderr: error instanceof Error ? error.message : String(error),
    };
  }
}

const describeIndex = process.argv.indexOf("--describe-platform");
if (describeIndex >= 0) {
  const platform = process.argv[describeIndex + 1];
  const requirements = REQUIREMENTS[platform];
  if (!requirements) {
    console.error(`Unknown platform: ${platform ?? ""}`);
    process.exit(2);
  }
  console.log(JSON.stringify({ platform, requirements }, null, 2));
} else {
  const report = inspect(process.platform);
  if (usageBoolean("json") || process.argv.includes("--json")) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(`System prerequisites (${report.platform}):`);
    for (const check of report.checks) {
      console.log(`  ${check.ok ? "PASS" : "FAIL"} ${check.name}`);
      if (check.hint) console.log(`       ${check.hint}`);
    }
  }
  if (!report.ok) process.exitCode = 1;
}
