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
const INSTALL_DIR_UI_FRAGMENT = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "wix",
  "fyagent-install-dir-ui.wxs",
);
const INSTALLER_ACTIONS_MANIFEST = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "installer-actions",
  "Cargo.toml",
);
const INSTALLER_ACTIONS_LIB = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "installer-actions",
  "src",
  "lib.rs",
);
const TAURI_CONFIG = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "tauri.conf.json",
);
const WINDOWS_MSI_VERIFIER = path.resolve(
  __dirname,
  "..",
  "scripts",
  "release",
  "verify-windows-msi.ps1",
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
    expect(releaseJobSection).toContain("    needs: version-contract");
    expect(releaseJobSection).toContain(
      "      APP_VERSION: ${{ needs.version-contract.outputs.app_version }}",
    );
    expect(releaseJobSection).toContain(
      "      RELEASE_TAG: ${{ needs.version-contract.outputs.release_tag }}",
    );
    expect(releaseJobSection).toContain(
      "      SOURCE_SHA: ${{ needs.version-contract.outputs.source_sha }}",
    );
    expect(releaseJobSection).toContain(
      'matrix: ${{ fromJSON(github.ref_type == \'tag\' && \'{"include":[{"os":"windows-2022"},{"os":"windows-11-arm","arch":"arm64"},{"os":"ubuntu-22.04"},{"os":"ubuntu-22.04-arm","arch":"arm64"},{"os":"macos-14"}]}\' || \'{"include":[{"os":"macos-14"}]}\') }}',
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

  it("freezes Cargo-derived version, tag, and source SHA before platform builds", () => {
    const versionContractStart = source.indexOf("  version-contract:\n");
    const releaseStart = source.indexOf("\n  release:\n");
    expect(versionContractStart).toBeGreaterThan(-1);
    expect(releaseStart).toBeGreaterThan(versionContractStart);
    const contract = source.slice(versionContractStart, releaseStart);

    expect(contract).toContain("pnpm run version:check");
    expect(contract).toContain(
      'pnpm run version:check -- --tag "${GITHUB_REF_NAME}"',
    );
    expect(contract).toContain(
      'app_version="$(pnpm --silent run version:get)"',
    );
    expect(contract).toContain('release_tag="v${app_version}"');
    expect(contract).toContain("printf 'source_sha=%s\\n' \"$GITHUB_SHA\"");
    expect(source).toContain('VERSION="$APP_VERSION"');
    expect(source).toContain("$VERSION = $env:APP_VERSION");
    expect(source).not.toContain('VERSION="${GITHUB_REF_NAME}"');
    expect(source).not.toContain("$VERSION = $env:GITHUB_REF_NAME");
    expect(source).toContain("Generate frozen download manifest");
    expect(source).toContain("$SOURCE_SHA");
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

  it("builds a target-matched installer helper before bundling and verifies the MSI tables", () => {
    const helperBuildIndex = source.indexOf(
      "- name: Build and verify Windows installer-actions DLL",
    );
    const bundleIndex = source.indexOf("- name: Bundle signed Windows MSI");
    const msiStructureIndex = source.indexOf(
      "- name: Verify Windows MSI native directory validator structure",
    );
    const payloadVerificationIndex = source.indexOf(
      "- name: Verify Windows MSI payload and installer-actions binding",
    );
    const msiSignIndex = source.indexOf("- name: Sign and verify Windows MSI");

    expect(helperBuildIndex).toBeGreaterThan(-1);
    expect(bundleIndex).toBeGreaterThan(helperBuildIndex);
    expect(msiStructureIndex).toBeGreaterThan(bundleIndex);
    expect(payloadVerificationIndex).toBeGreaterThan(msiStructureIndex);
    expect(msiSignIndex).toBeGreaterThan(payloadVerificationIndex);
    expect(source).toContain("--package fyagent-installer-actions");
    expect(source).toContain("aarch64-pc-windows-msvc");
    expect(source).toContain("FYAGENT_INSTALLER_ACTIONS_DLL");
    expect(source).toContain("TAURI_FYAGENT_INSTALLER_ACTIONS_DLL");
    expect(source).toContain("0xAA64");
    expect(source).toContain("0x8664");
    expect(source).toContain("FyAgentInstallerActions");
    expect(source).toContain("WindowsInstaller.Installer");
    expect(source).toContain("ValidateFyAgentInstallDirUi");
    expect(source).toContain("ValidateFyAgentInstallDirExecute");
    expect(source).toContain("AbortUnsafeFyAgentInstallDir");
    expect(source).toContain("ClearFyAgentPreviousInstallDir");
    expect(source).toContain("ClearMaintenanceInstallDir");
    expect(source).toContain("RestoreInstallDirFromPrevious");
    expect(source).toContain("AbortUntrustedFyAgentMaintenance");
    expect(source).toContain("MSI INSTALLDIR protected DACL contract drifted");
    expect(source).toContain("MSI INSTALLDIR component guard drifted");
    expect(source).toContain("Get-MsiRecords");
    expect(source).toContain("MSI sequence order failed");
  });

  it("extracts the MSI Binary stream and binds it to the architecture-matched helper", () => {
    const verifier = fs.readFileSync(WINDOWS_MSI_VERIFIER, "utf8");

    expect(source).toContain("./scripts/release/verify-windows-msi.ps1");
    expect(source).toContain(
      "-InstallerActionsDll $env:FYAGENT_INSTALLER_ACTIONS_DLL",
    );
    expect(source).toContain("-Architecture $(if ($isArm64)");
    expect(source).toContain("-AppVersion $env:APP_VERSION");
    expect(verifier).toContain("OpenDatabase($resolvedMsi, 0)");
    expect(verifier).toContain("Binary.FyAgentInstallerActions");
    expect(verifier).toContain("$binaryRecord.DataSize(1)");
    expect(verifier).toContain("$binaryRecord.ReadStream(1, $requested, 1)");
    expect(verifier).toContain("[Text.Encoding]::GetEncoding(28591)");
    expect(verifier).toContain("0x8664");
    expect(verifier).toContain("0xAA64");
    expect(verifier).toContain("Get-FileHash");
    expect(verifier).toContain("ProductName must be FyAgent");
    expect(verifier).toContain("ARPNOREPAIR must be 1 or yes");
    expect(verifier).toContain(
      "complete fyagent URL protocol registry contract",
    );
    expect(verifier).toContain("Path payload ending in fyagent.exe");
    expect(verifier).toContain("URL:FyAgent protocol");
    expect(verifier).toContain("$database.SummaryInformation(0)");
    expect(verifier).toContain("0x2000");
    expect(verifier).toContain(
      "Linux or retired cross-build host-path residue",
    );
    expect(verifier).toContain("FinalReleaseComObject");
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
    expect(source).toContain(
      "name: FyAgent ${{ needs.version-contract.outputs.release_tag }}",
    );
    expect(source).toContain('--volname "FyAgent"');
  });

  it("documents manual release delivery rather than an application updater", () => {
    const readmeSource = fs.readFileSync(README, "utf8");
    const readme = readmeSource.toLowerCase();

    expect(readme).toContain("manual release downloads");
    expect(readme).not.toContain("auto-updater");
    expect(readme).not.toContain("tauri-plugin-updater");
    expect(readmeSource).toContain("FyAgent-{version}-Linux-{arch}.AppImage");
    expect(readmeSource).toContain("FyAgent-{version}-Linux-{arch}.deb");
    expect(readmeSource).toContain("FyAgent-{version}-Linux-{arch}.rpm");
  });
});

describe("FyAgent Windows elevation and installer boundary", () => {
  const source = fs.readFileSync(PER_MACHINE_WIX_TEMPLATE, "utf8");
  const releaseSource = fs.readFileSync(RELEASE_WORKFLOW, "utf8");
  const installDirUi = fs.readFileSync(INSTALL_DIR_UI_FRAGMENT, "utf8");
  const cargoToml = fs.readFileSync(CARGO_TOML, "utf8");
  const installerActionsManifest = fs.readFileSync(
    INSTALLER_ACTIONS_MANIFEST,
    "utf8",
  );
  const installerActionsLib = fs.readFileSync(INSTALLER_ACTIONS_LIB, "utf8");
  const buildRs = fs.readFileSync(BUILD_RS, "utf8");
  const testManifest = fs.readFileSync(TEST_MANIFEST, "utf8");
  const releaseManifest = fs.readFileSync(RELEASE_MANIFEST, "utf8");
  const autoLaunch = fs.readFileSync(AUTO_LAUNCH, "utf8");
  const libRs = fs.readFileSync(LIB_RS, "utf8");
  const ciWorkflow = fs.readFileSync(CI_WORKFLOW, "utf8");
  const windowsMsiVerifier = fs.readFileSync(WINDOWS_MSI_VERIFIER, "utf8");

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
    expect(buildRs).toContain("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    expect(buildRs).toContain("cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}");
    expect(buildRs).not.toContain("cargo:rustc-link-arg=/MANIFEST:EMBED");
    expect(buildRs).not.toContain("cargo:rustc-link-arg=/MANIFESTINPUT:");
    expect(buildRs).not.toContain("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    expect(buildRs).toContain("windows/fyagent-test.manifest");
    expect(buildRs).toContain('PROFILE").as_deref() == Ok("release")');
    expect(buildRs).toContain(
      "FYAGENT_WINDOWS_MANIFEST must be explicitly set to release",
    );
    expect(ciWorkflow).toContain("FYAGENT_WINDOWS_MANIFEST: test");
    expect(releaseSource).toContain("FYAGENT_WINDOWS_MANIFEST: release");
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
    expect(source).not.toContain('<UIRef Id="WixUI_InstallDir" />');
    expect(source).toContain('<UIRef Id="FyAgent_InstallDir" />');
    expect(installDirUi).toContain('<UI Id="FyAgent_InstallDir">');
    expect(installDirUi).toContain('<UIRef Id="WixUI_Common" />');
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
    expect(
      JSON.parse(fs.readFileSync(TAURI_CONFIG, "utf8")).bundle.windows.wix
        .fragmentPaths,
    ).toEqual(["wix/fyagent-install-dir-ui.wxs"]);
    expect(releaseSource).toContain("WindowsInstaller.Installer");
  });

  it("uses an architecture-matched native Type 1 validator in UI and execute paths", () => {
    expect(source).toContain(
      '<Binary Id="FyAgentInstallerActions" SourceFile="$(env.TAURI_FYAGENT_INSTALLER_ACTIONS_DLL)" />',
    );
    expect(source).toContain('Id="ValidateFyAgentInstallDirUi"');
    expect(source).toContain('DllEntry="ValidateFyAgentInstallDirUi"');
    expect(source).toContain('Id="ValidateFyAgentInstallDirExecute"');
    expect(source).toContain('DllEntry="ValidateFyAgentInstallDirExecute"');
    expect(source).toContain('Id="ApplyValidatedFyAgentInstallDir"');
    expect(source).toContain('Property="INSTALLDIR"');
    expect(source).toContain('Value="[FYAGENT_INSTALLDIR_NORMALIZED]"');
    expect(source).toContain('Id="AbortUnsafeFyAgentInstallDir"');
    expect(source).toContain('Error="[FYAGENT_INSTALLDIR_ERROR_MESSAGE]"');
    expect(source).toContain("ValidateFyAgentInstallDirUi");
    expect(source).toContain("ValidateFyAgentInstallDirExecute");
    expect(source).toContain("AbortUnsafeFyAgentInstallDir");
    expect(source).toContain(
      "NOT ($CMP_UninstallShortcut = 2 AND $InstallDirectoryAcl = 2 AND $Path = 2 AND $RegistryEntries = 2)",
    );
    expect(source).not.toContain('REMOVE~="ALL" AND NOT REINSTALL');
    expect(source).toContain('Property Id="FYAGENT_PREVIOUS_INSTALLDIR"');
    expect(source).toContain(
      'Id="FYAGENT_PREVIOUS_INSTALLDIR" Action="ClearFyAgentPreviousInstallDir" Value="" Before="AppSearch" Sequence="first"',
    );
    expect(source).toContain(
      'Id="INSTALLDIR" Action="ClearMaintenanceInstallDir" Value="" Before="AppSearch" Sequence="first"',
    );
    expect(source).toContain(
      'Id="INSTALLDIR" Action="RestoreInstallDirFromPrevious" Value="[FYAGENT_PREVIOUS_INSTALLDIR]" Before="CostFinalize" Sequence="both"',
    );
    expect(source).toContain('Root="HKLM"');
    expect(source).toContain("WIX_UPGRADE_DETECTED");
    expect(source).toContain('Id="AbortUntrustedFyAgentMaintenance"');
    expect(source).toContain('Value="URL:FyAgent protocol"');
    expect(source).toContain(
      "(Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE) AND NOT FYAGENT_PREVIOUS_INSTALLDIR AND NOT ($CMP_UninstallShortcut = 2 AND $InstallDirectoryAcl = 2 AND $Path = 2 AND $RegistryEntries = 2)",
    );
    expect(source).toContain(
      'Action="AbortUntrustedFyAgentMaintenance" Before="ValidateFyAgentInstallDirExecute"',
    );

    for (const property of [
      "INSTALLDIR",
      "FYAGENT_INSTALLDIR_VALID",
      "FYAGENT_INSTALLDIR_ERROR_CODE",
      "FYAGENT_INSTALLDIR_ERROR_MESSAGE",
      "FYAGENT_INSTALLDIR_NORMALIZED",
      "FYAGENT_INSTALLDIR_CHECK_ID",
    ]) {
      expect(source).toContain(`<Property Id="${property}" Secure="yes" />`);
    }

    for (const forbiddenLegacyMarker of [
      'Script="vbscript"',
      "ValidateInstallDirectory",
      "Scripting.FileSystemObject",
      "Win32_LogicalFileSecuritySetting",
      "GetSecurityDescriptor",
      "Err.Raise",
    ]) {
      expect(source).not.toContain(forbiddenLegacyMarker);
    }

    expect(installerActionsManifest).toContain('crate-type = ["cdylib"]');
    expect(installerActionsManifest).toContain(
      'windows-sys = { version = "0.61"',
    );
    expect(installerActionsLib).toContain(
      "ValidateFyAgentInstallDirUi(install: MSIHANDLE)",
    );
    expect(installerActionsLib).toContain(
      "ValidateFyAgentInstallDirExecute(install: MSIHANDLE)",
    );
    expect(installerActionsLib).toContain("ERROR_SUCCESS");
    expect(installerActionsLib).toContain("ERROR_INSTALL_FAILURE");
  });

  it("orders native validation after standard path validation and keeps policy denial recoverable", () => {
    const setTargetPath = installDirUi.indexOf('Event="SetTargetPath"');
    const standardValidation = installDirUi.indexOf(
      'Value="WixUIValidatePath" Order="2"',
    );
    const standardFailure = installDirUi.indexOf(
      'Value="InvalidDirDlg" Order="3"',
    );
    const nativeValidation = installDirUi.indexOf(
      'Value="ValidateFyAgentInstallDirUi" Order="4"',
    );
    const applyNormalizedDirectory = installDirUi.indexOf(
      'Value="ApplyValidatedFyAgentInstallDir" Order="5"',
    );
    const policyFailure = installDirUi.indexOf(
      'Value="FyAgentUnsafeInstallDirDlg" Order="6"',
    );
    const success = installDirUi.indexOf('Value="VerifyReadyDlg" Order="7"');

    expect(setTargetPath).toBeGreaterThan(-1);
    expect(standardValidation).toBeGreaterThan(setTargetPath);
    expect(standardFailure).toBeGreaterThan(standardValidation);
    expect(nativeValidation).toBeGreaterThan(standardFailure);
    expect(applyNormalizedDirectory).toBeGreaterThan(nativeValidation);
    expect(policyFailure).toBeGreaterThan(applyNormalizedDirectory);
    expect(success).toBeGreaterThan(policyFailure);
    expect(installDirUi).toContain('Id="FyAgentUnsafeInstallDirDlg"');
    expect(installDirUi).toContain("[FYAGENT_INSTALLDIR_ERROR_MESSAGE]");
    expect(installDirUi).toContain('Event="EndDialog" Value="Return"');
    expect(installDirUi).toContain("WIX_UPGRADE_DETECTED");
    expect(installDirUi).toContain(
      "NOT ($CMP_UninstallShortcut = 2 AND $InstallDirectoryAcl = 2 AND $Path = 2 AND $RegistryEntries = 2)",
    );
    expect(releaseSource).toContain("FYAGENT_INSTALLER_ACTIONS_DLL");
    expect(releaseSource).toContain("TAURI_FYAGENT_INSTALLER_ACTIONS_DLL");
    expect(releaseSource).toContain("fyagent_installer_actions.dll");
    expect(releaseSource).toContain("MsiLockPermissionsEx");
    expect(releaseSource).toContain("Test-InstallDirDescendant");
    expect(releaseSource).toContain("Assert-MsiCustomAction");
    expect(releaseSource).toContain("ValidateFyAgentInstallDirExecute");
    expect(windowsMsiVerifier).toContain(
      "must not install the custom-action DLL as application payload",
    );
    expect(releaseSource).toContain(
      "$installerActionsTarget = Join-Path $PWD 'src-tauri/target/installer-actions'",
    );
    expect(releaseSource).toContain(
      "$env:CARGO_TARGET_DIR = $installerActionsTarget",
    );
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
    expect(autoLaunch).toContain(
      [
        "pub fn is_auto_launch_enabled() -> Result<bool, AppError> {",
        '    #[cfg(target_os = "windows")]',
        "    {",
        "        clear_windows_auto_launch_entry()?;",
        "        Ok(false)",
        "    }",
      ].join("\n"),
    );
    expect(autoLaunch).not.toContain(
      'WINDOWS_AUTO_LAUNCH_VALUE: &str = "CC Switch"',
    );
    const cleanupIndex = libRs.indexOf(
      "auto_launch::enforce_platform_auto_launch_policy()",
    );
    const builderIndex = libRs.indexOf(
      "let builder = tauri::Builder::default();",
    );
    expect(cleanupIndex).toBeGreaterThan(-1);
    expect(builderIndex).toBeGreaterThan(-1);
    expect(cleanupIndex).toBeLessThan(builderIndex);
  });

  it("keeps cdylib output isolated to the installer helper", () => {
    expect(cargoToml).toContain('crate-type = ["staticlib", "rlib"]');
    expect(cargoToml).not.toContain('"cdylib"');
    expect(installerActionsManifest).toContain('crate-type = ["cdylib"]');
  });
});
