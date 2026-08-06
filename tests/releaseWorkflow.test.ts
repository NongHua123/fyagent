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
const BUILD_RS = path.resolve(__dirname, "..", "src-tauri", "build.rs");
const TEST_MANIFEST = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "windows",
  "fyagent-test.manifest",
);
const RELEASE_MANIFEST = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "windows",
  "fyagent-release.manifest",
);
const PER_MACHINE_WIX_TEMPLATE = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "wix",
  "per-machine-main.wxs",
);
const TAURI_CONFIG = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "tauri.conf.json",
);
const WINDOWS_CROSS_BUILD = path.resolve(
  __dirname,
  "..",
  "scripts",
  "windows-cross",
  "build-windows-msi.sh",
);
const AUTO_LAUNCH = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "src",
  "auto_launch.rs",
);
const LIB_RS = path.resolve(__dirname, "..", "src-tauri", "src", "lib.rs");
const CI_WORKFLOW = path.resolve(
  __dirname,
  "..",
  ".github",
  "workflows",
  "ci.yml",
);

describe("FyAgent release workflow", () => {
  const source = fs.readFileSync(RELEASE_WORKFLOW, "utf8");

  it("builds the unsigned macOS branch DMG only by manual dispatch", () => {
    const normalizedWorkflow = source.replace(/\r\n/g, "\n");
    const triggerSection = normalizedWorkflow
      .slice(0, normalizedWorkflow.indexOf("\npermissions:"))
      .trimEnd();
    const releaseJobStart = normalizedWorkflow.indexOf("\n  release:\n");
    const publishJobStart = normalizedWorkflow.indexOf(
      "\n  publish-release:\n",
    );
    expect(releaseJobStart).toBeGreaterThan(-1);
    expect(publishJobStart).toBeGreaterThan(releaseJobStart);
    const releaseJobSection = normalizedWorkflow.slice(
      releaseJobStart,
      publishJobStart,
    );
    const publishJobSection = normalizedWorkflow.slice(publishJobStart);
    const releaseJobHeader = releaseJobSection
      .slice(1, releaseJobSection.indexOf("\n    steps:"))
      .trimEnd();

    expect(triggerSection).toBe(
      [
        "name: Release",
        "",
        "on:",
        "  push:",
        "    tags:",
        '      - "v*"',
        "  workflow_dispatch:",
      ].join("\n"),
    );
    expect(releaseJobHeader).toBe(
      [
        "  release:",
        "    if: github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && github.ref_type == 'branch')",
        "    runs-on: ${{ matrix.os }}",
        "    environment: release",
        "    strategy:",
        "      fail-fast: false",
        '      matrix: ${{ fromJSON(github.ref_type == \'tag\' && \'{"include":[{"os":"windows-2022"},{"os":"windows-11-arm","arch":"arm64"},{"os":"ubuntu-22.04"},{"os":"ubuntu-22.04-arm","arch":"arm64"},{"os":"macos-14"}]}\' || \'{"include":[{"os":"macos-14"}]}\') }}',
      ].join("\n"),
    );
    expect(releaseJobSection).toContain(
      "Build unsigned Tauri App (macOS branch)",
    );
    expect(publishJobSection).toContain(
      "  publish-release:\n    name: Publish GitHub Release\n    if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')",
    );
  });

  it("publishes only manual-install assets without an updater chain", () => {
    const normalizedSource = source.toLowerCase();

    expect(source).not.toContain("TAURI_SIGNING_PRIVATE_KEY");
    expect(normalizedSource).not.toContain("updater");
    expect(normalizedSource).not.toContain("latest.json");
    expect(normalizedSource).not.toContain(".tar.gz");
    expect(normalizedSource).not.toMatch(/\.sig["'\s]/);
    expect(normalizedSource).not.toContain("portable");
    expect(normalizedSource).not.toContain("compress-archive");
    expect(source).toContain("FyAgent-${VERSION}-macOS.dmg");
    expect(source).toContain("FyAgent-$VERSION-Windows$assetSuffix.msi");
    expect(source).toContain(
      'NEW_APPIMAGE="FyAgent-${VERSION}-Linux-${ARCH}.AppImage"',
    );
    expect(source).toContain(
      '"release-assets/FyAgent-${VERSION}-Linux-${ARCH}.deb"',
    );
    expect(source).toContain(
      '"release-assets/FyAgent-${VERSION}-Linux-${ARCH}.rpm"',
    );
  });

  it("fails closed unless Windows EXE and MSI signing includes a timestamp", () => {
    expect(source).toContain("FYAGENT_WINDOWS_MANIFEST: release");
    expect(source).toContain("WINDOWS_AUTHENTICODE_PFX_BASE64:");
    expect(source).toContain("secrets.WINDOWS_AUTHENTICODE_PFX_BASE64");
    expect(source).toContain("WINDOWS_AUTHENTICODE_PUBLISHER_SUBJECT:");
    expect(source).toContain("secrets.WINDOWS_AUTHENTICODE_PUBLISHER_SUBJECT");
    expect(source).toContain("WINDOWS_AUTHENTICODE_TIMESTAMP_URL:");
    expect(source).toContain("secrets.WINDOWS_AUTHENTICODE_TIMESTAMP_URL");
    expect(source).toContain("pnpm tauri build --no-bundle");
    expect(source).toContain("pnpm tauri bundle --bundles msi");
    expect(source).toContain("signtool.exe sign");
    expect(source).toContain("/tr $env:WINDOWS_AUTHENTICODE_TIMESTAMP_URL");
    expect(source).toContain("Get-AuthenticodeSignature");
    expect(source).toContain("FYAGENT_WINDOWS_CERT_PUBLISHER_SUBJECT");
    expect(source).toContain("does not match the expected publisher subject");
    expect(source).toContain("TimeStamperCertificate");
    expect(source).toContain("No signed Windows MSI installer found");
  });

  it("extracts and checks the embedded elevated manifest before signing and after MSI bundling", () => {
    const buildIndex = source.indexOf("- name: Build Tauri App (Windows)");
    const beforeExeSignIndex = source.indexOf(
      "- name: Verify embedded Windows release manifest before EXE signing",
    );
    const exeSignIndex = source.indexOf(
      "- name: Sign and verify Windows application executable",
    );
    const bundleIndex = source.indexOf("- name: Bundle signed Windows MSI");
    const afterBundleIndex = source.indexOf(
      "- name: Re-verify embedded Windows release manifest after MSI bundle",
    );
    const msiSignIndex = source.indexOf("- name: Sign and verify Windows MSI");

    expect(buildIndex).toBeGreaterThan(-1);
    expect(beforeExeSignIndex).toBeGreaterThan(buildIndex);
    expect(exeSignIndex).toBeGreaterThan(beforeExeSignIndex);
    expect(bundleIndex).toBeGreaterThan(exeSignIndex);
    expect(afterBundleIndex).toBeGreaterThan(bundleIndex);
    expect(msiSignIndex).toBeGreaterThan(afterBundleIndex);

    const manifestCheckSections = [
      source.slice(beforeExeSignIndex, exeSignIndex),
      source.slice(afterBundleIndex, msiSignIndex),
    ];
    for (const section of manifestCheckSections) {
      expect(section).toContain("mt.exe");
      expect(section).toContain("RT_MANIFEST");
      expect(section).toContain("-inputresource:$exePath;#1");
      expect(section).toContain("requestedExecutionLevel");
      expect(section).toContain("Count -ne 1");
      expect(section).toContain(
        "$requestedExecutionLevel.GetAttribute('level') -cne 'requireAdministrator'",
      );
      expect(section).toContain(
        "$requestedExecutionLevel.GetAttribute('uiAccess') -cne 'false'",
      );
      expect(section).toContain("Attributes.Count -ne 2");
    }
  });

  it("does not reintroduce old public branding or website links", () => {
    expect(source).not.toContain("CC Switch");
    expect(source).not.toContain("CC-Switch");
    expect(source).not.toContain("ccswitch.io");
    expect(source).not.toContain("cc-switch.exe");
    expect(source).toContain("fyagent.exe");
    expect(source).toContain("name: FyAgent ${{ github.ref_name }}");
    expect(source).toContain('--volname "FyAgent"');
  });

  it("documents manual release delivery rather than an application updater", () => {
    const readmeSource = fs.readFileSync(README, "utf8");
    const readme = readmeSource.toLowerCase();

    expect(readme).toContain("manual release downloads");
    expect(readme).not.toContain("auto-updater");
    expect(readme).not.toContain("tauri-plugin-updater");
    expect(readmeSource).toContain("FyAgent-v{version}-Linux-{arch}.AppImage");
    expect(readmeSource).toContain("FyAgent-v{version}-Linux-{arch}.deb");
    expect(readmeSource).toContain("FyAgent-v{version}-Linux-{arch}.rpm");
  });
});

describe("FyAgent Windows elevation and installer boundary", () => {
  const source = fs.readFileSync(PER_MACHINE_WIX_TEMPLATE, "utf8");
  const cargoToml = fs.readFileSync(CARGO_TOML, "utf8");
  const buildRs = fs.readFileSync(BUILD_RS, "utf8");
  const testManifest = fs.readFileSync(TEST_MANIFEST, "utf8");
  const releaseManifest = fs.readFileSync(RELEASE_MANIFEST, "utf8");
  const autoLaunch = fs.readFileSync(AUTO_LAUNCH, "utf8");
  const libRs = fs.readFileSync(LIB_RS, "utf8");
  const ciWorkflow = fs.readFileSync(CI_WORKFLOW, "utf8");

  it("selects normal-privilege test and elevated release manifests", () => {
    expect(testManifest).toContain(
      '<requestedExecutionLevel level="asInvoker" uiAccess="false" />',
    );
    expect(testManifest).not.toContain("requireAdministrator");
    expect(releaseManifest).toContain(
      '<requestedExecutionLevel level="requireAdministrator" uiAccess="false" />',
    );
    for (const manifest of [testManifest, releaseManifest]) {
      expect(manifest).toContain("Microsoft.Windows.Common-Controls");
      expect(manifest).toContain('version="6.0.0.0"');
    }
    expect(buildRs).toContain("CARGO_CFG_TARGET_OS");
    expect(buildRs).toContain("FYAGENT_WINDOWS_MANIFEST");
    expect(buildRs).toContain("WindowsAttributes::new().app_manifest");
    expect(buildRs).toContain(
      "cargo:rustc-check-cfg=cfg(fyagent_windows_release)",
    );
    expect(buildRs).toContain("cargo:rustc-cfg=fyagent_windows_release");
    expect(buildRs).toContain("WindowsManifest::Release");
    expect(buildRs).toContain("cargo:rustc-link-arg=/MANIFEST:EMBED");
    expect(buildRs).toContain("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    expect(buildRs).toContain("windows/fyagent-test.manifest");
    expect(buildRs).toContain('PROFILE").as_deref() == Ok("release")');
    expect(buildRs).toContain(
      "FYAGENT_WINDOWS_MANIFEST must be explicitly set to release",
    );
    expect(ciWorkflow).toContain("FYAGENT_WINDOWS_MANIFEST: test");
    expect(
      fs.existsSync(
        path.resolve(__dirname, "..", "src-tauri", "common-controls.manifest"),
      ),
    ).toBe(false);
  });

  it("uses a per-machine WiX template with no LocalAppData install root", () => {
    expect(source).toContain('InstallScope="perMachine"');
    expect(source).toContain('InstallPrivileges="elevated"');
    expect(source).toContain(
      '<Directory Id="$(var.PlatformProgramFilesFolder)">',
    );
    expect(source).toContain('<UIRef Id="WixUI_InstallDir" />');
    expect(source).not.toContain('InstallScope="perUser"');
    expect(source).not.toContain("TauriLocalAppDataPrograms");
    expect(source).not.toContain("per-user-main.wxs");
    expect(
      fs.existsSync(
        path.resolve(__dirname, "..", "src-tauri", "wix", "per-user-main.wxs"),
      ),
    ).toBe(false);
    expect(source).toContain("LEGACY_FYAGENT_PER_USER_INSTALLDIR");
    expect(source).toContain("Uninstall it manually before installing");
    expect(source).toContain('Root="HKLM"');
    expect(source).toContain(
      '<File Id="Bin_{{ bin.id }}" Source="{{bin.path}}" KeyPath="no"/>',
    );
    expect(source).toContain(
      'Name="BundledBinary_{{ bin.id }}" Type="integer" Value="1" KeyPath="yes"',
    );
    expect(
      JSON.parse(fs.readFileSync(TAURI_CONFIG, "utf8")).bundle.windows.wix
        .template,
    ).toBe("wix/per-machine-main.wxs");
    expect(fs.readFileSync(WINDOWS_CROSS_BUILD, "utf8")).toContain(
      "src-tauri/wix/per-machine-main.wxs",
    );
  });

  it("makes the custom directory validator fail closed for unsafe locations", () => {
    expect(source).toContain('CustomAction Id="ValidateInstallDirectory"');
    expect(source).toContain('Script="vbscript"');
    expect(source).toContain("InstallUISequence");
    expect(source).toContain("InstallExecuteSequence");
    expect(source).toContain('After="CostFinalize">NOT Installed</Custom>');
    expect(source).toContain("DRIVE_FIXED");
    expect(source).toContain("FILE_ATTRIBUTE_REPARSE_POINT");
    expect(source).toContain("Win32_LogicalFileSecuritySetting");
    expect(source).toContain("ACCESS_GENERIC_WRITE");
    expect(source).toContain("SID_TRUSTED_INSTALLER");
    expect(source).toContain("descriptor.Owner");
    expect(source).toContain("ACE_TYPE_ACCESS_ALLOWED");
    expect(source).toContain("ACE_TYPE_ACCESS_DENIED");
    expect(source).toContain("IsTrustedAclPrincipal");
    expect(source).toContain("not provably administrator controlled");
    expect(source).toContain("UserProfileFolder");
    expect(source).toContain("DesktopFolder");
    expect(source).toContain("LocalAppDataFolder");
    expect(source).toContain("Win32_LogicalFileSecuritySetting");
  });

  it("provisions a protected ProgramData root for elevated activation state", () => {
    expect(source).toContain('<Directory Id="CommonAppDataFolder">');
    expect(source).toContain(
      '<Directory Id="FYAGENTCOMMONDATA" Name="FyAgent">',
    );
    expect(source).toContain('<Directory Id="FYAGENTRUNTIME" Name="runtime"/>');
    expect(source).toContain('<DirectoryRef Id="FYAGENTCOMMONDATA">');
    expect(source).toContain('<DirectoryRef Id="FYAGENTRUNTIME">');
    expect(source).toContain(
      '<PermissionEx Sddl="O:SYD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)" />',
    );
    expect(source).toContain(
      '<ComponentRef Id="RuntimeStateParentDirectoryAcl"/>',
    );
    expect(source).toContain('<ComponentRef Id="RuntimeStateDirectoryAcl"/>');
    expect(source).toContain(
      "O:SYD:P(A;OICI;0x1200a9;;;BU)(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)",
    );
    expect(source).not.toContain("<Permission User=");
  });

  it("disables Windows autolaunch and only clears FyAgent's own legacy value", () => {
    expect(autoLaunch).toContain(
      'const WINDOWS_AUTO_LAUNCH_VALUE: &str = "FyAgent";',
    );
    expect(autoLaunch).toContain("clear_windows_auto_launch_entry");
    expect(autoLaunch).toContain("enforce_platform_auto_launch_policy");
    expect(autoLaunch).toContain("Windows 版本已禁用开机自启");
    expect(autoLaunch).toContain("return Ok(false);");
    expect(autoLaunch).not.toContain(
      'WINDOWS_AUTO_LAUNCH_VALUE: &str = "CC Switch"',
    );
    const cleanupIndex = libRs.indexOf(
      "auto_launch::enforce_platform_auto_launch_policy()",
    );
    expect(cleanupIndex).toBeGreaterThan(-1);
    expect(cleanupIndex).toBeLessThan(
      libRs.indexOf("let mut builder = tauri::Builder::default();"),
    );
  });

  it("does not produce an unused cdylib for the desktop-only Windows MSI", () => {
    expect(cargoToml).toContain('crate-type = ["staticlib", "rlib"]');
    expect(cargoToml).not.toContain('"cdylib"');
  });
});
