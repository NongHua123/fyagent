import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(__dirname, "..");
const packageJsonPath = path.join(repositoryRoot, "package.json");
const cargoTomlPath = path.join(repositoryRoot, "src-tauri", "Cargo.toml");
const tauriConfigPath = path.join(
  repositoryRoot,
  "src-tauri",
  "tauri.conf.json",
);
const FYAGENT_V1_0_2_VERSION = "0.2.0";

// SemVer 2.0.0: numeric prerelease identifiers cannot contain leading zeroes,
// while non-numeric identifiers and build metadata may contain ASCII hyphens.
const semverPattern =
  /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

function readCargoPackageVersion(content: string): string {
  const packageSection = content.match(
    /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
  );

  if (!packageSection?.[1]) {
    throw new Error("Cargo package version is missing");
  }

  return packageSection[1];
}

describe("FyAgent application version metadata", () => {
  it("keeps npm, Cargo, and Tauri on the locked v1-0.2 SemVer version", () => {
    const packageVersion = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"))
      .version as string;
    const cargoVersion = readCargoPackageVersion(
      fs.readFileSync(cargoTomlPath, "utf8"),
    );
    const tauriVersion = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"))
      .version as string;

    expect(packageVersion).toMatch(semverPattern);
    expect(packageVersion).toBe(FYAGENT_V1_0_2_VERSION);
    expect(cargoVersion).toBe(packageVersion);
    expect(tauriVersion).toBe(packageVersion);
  });
});
