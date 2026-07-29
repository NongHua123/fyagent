import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const RELEASE_WORKFLOW = path.resolve(
  __dirname,
  "..",
  ".github",
  "workflows",
  "release.yml",
);
const README = path.resolve(__dirname, "..", "README.md");
const CARGO_TOML = path.resolve(__dirname, "..", "src-tauri", "Cargo.toml");
const PER_USER_WIX_TEMPLATE = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "wix",
  "per-user-main.wxs",
);

describe("FyAgent release workflow", () => {
  const source = fs.readFileSync(RELEASE_WORKFLOW, "utf8");

  it("publishes only manual-install assets without an updater chain", () => {
    const normalizedSource = source.toLowerCase();

    expect(source).not.toContain("TAURI_SIGNING_PRIVATE_KEY");
    expect(normalizedSource).not.toContain("updater");
    expect(normalizedSource).not.toContain("latest.json");
    expect(normalizedSource).not.toContain(".tar.gz");
    expect(normalizedSource).not.toContain(".sig");
    expect(source).toContain("FyAgent-${VERSION}-macOS.dmg");
    expect(source).toContain("FyAgent-$VERSION-Windows$assetSuffix.msi");
  });

  it("does not reintroduce old public branding or website links", () => {
    expect(source).not.toContain("CC Switch");
    expect(source).not.toContain("CC-Switch");
    expect(source).not.toContain("ccswitch.io");
    expect(source).toContain("name: FyAgent ${{ github.ref_name }}");
    expect(source).toContain('--volname "FyAgent"');
  });

  it("documents manual release delivery rather than an application updater", () => {
    const readme = fs.readFileSync(README, "utf8").toLowerCase();

    expect(readme).toContain("manual release downloads");
    expect(readme).not.toContain("auto-updater");
    expect(readme).not.toContain("tauri-plugin-updater");
  });
});

describe("FyAgent per-user WiX template", () => {
  const source = fs.readFileSync(PER_USER_WIX_TEMPLATE, "utf8");
  const cargoToml = fs.readFileSync(CARGO_TOML, "utf8");

  it("uses HKCU registry key paths for bundled binaries", () => {
    expect(source).toContain('InstallScope="perUser"');
    expect(source).toContain(
      '<File Id="Bin_{{ bin.id }}" Source="{{bin.path}}" KeyPath="no"/>',
    );
    expect(source).toContain(
      'Name="BundledBinary_{{ bin.id }}" Type="integer" Value="1" KeyPath="yes"',
    );
    expect(source).not.toContain(
      '<File Id="Bin_{{ bin.id }}" Source="{{bin.path}}" KeyPath="yes"/>',
    );
  });

  it("does not produce an unused cdylib for the desktop-only per-user MSI", () => {
    expect(cargoToml).toContain('crate-type = ["staticlib", "rlib"]');
    expect(cargoToml).not.toContain('"cdylib"');
  });
});
