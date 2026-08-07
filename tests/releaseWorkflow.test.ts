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
const WINDOWS_MSI_STRUCTURE_VERIFIER = path.resolve(
  __dirname,
  "..",
  "scripts",
  "release",
  "verify-windows-msi-structure.ps1",
);
const WINDOWS_MANIFEST_VERIFIER = path.resolve(
  __dirname,
  "..",
  "scripts",
  "release",
  "verify-windows-release-manifest.ps1",
);
const WINDOWS_UNSIGNED_VERIFIER = path.resolve(
  __dirname,
  "..",
  "scripts",
  "release",
  "verify-windows-unsigned.ps1",
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
  const windowsMsiVerifier = fs.readFileSync(WINDOWS_MSI_VERIFIER, "utf8");
  const windowsMsiStructureVerifier = fs.readFileSync(
    WINDOWS_MSI_STRUCTURE_VERIFIER,
    "utf8",
  );
  const windowsManifestVerifier = fs.readFileSync(
    WINDOWS_MANIFEST_VERIFIER,
    "utf8",
  );
  const windowsUnsignedVerifier = fs.readFileSync(
    WINDOWS_UNSIGNED_VERIFIER,
    "utf8",
  );

  it("supports only an immutable unsigned preflight and the exact v0.3.0 tag", () => {
    const trigger = source.slice(0, source.indexOf("\npermissions:"));
    expect(trigger).toContain('      - "v0.3.0"');
    expect(trigger).not.toContain('"v*"');
    expect(trigger).toContain("workflow_dispatch:");
    expect(trigger).toContain("source_sha:");
    expect(trigger).toContain("required: true");
    expect(source).toContain("release_mode='preflight'");
    expect(source).toContain("release_mode='formal'");
    expect(source).toContain(
      "if: needs.eligibility.outputs.release_mode == 'formal'",
    );
    expect(source).not.toContain("gh release create");
    expect(source).toContain("draft:true,prerelease:false");
    expect(source).toContain("draft:false,prerelease:false");
  });

  it("pins every third-party Action and every runner without latest labels", () => {
    const actionRefs = [...source.matchAll(/uses:\s+([^\s#]+)/g)].map(
      ([, reference]) => reference,
    );
    expect(actionRefs.length).toBeGreaterThan(0);
    for (const reference of actionRefs) {
      expect(reference).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
    }
    for (const runner of [
      "ubuntu-24.04",
      "ubuntu-24.04-arm",
      "windows-2022",
      "windows-11-arm",
      "macos-15",
    ]) {
      expect(source).toContain(runner);
    }
    expect(source).not.toMatch(/runs-on:\s*[^\n]*-latest/);
    expect(source).not.toContain("cache:");
    expect(source).not.toContain("actions/cache");
    expect(source.match(/uses: actions\/checkout@/g)).toHaveLength(
      source.match(/persist-credentials: false/g)?.length ?? 0,
    );
  });

  it("uses read-only defaults and isolates attestation and publish writes", () => {
    expect(source).toContain("permissions:\n  contents: read");
    expect(source).not.toContain("environment:");
    expect(source).toContain("artifact-metadata: write");
    expect(source).toContain("attestations: write");
    expect(source).toContain("id-token: write");
    const publish = source.slice(source.indexOf("\n  publish:\n"));
    expect(publish).toContain("contents: write");
    expect(source.slice(0, source.indexOf("\n  publish:\n"))).not.toContain(
      "contents: write",
    );
  });

  it("binds repository, workflow, tag, main ancestry, and one successful Required CI attempt", () => {
    const eligibility = source.slice(
      source.indexOf("\n  eligibility:\n"),
      source.indexOf("\n  build-windows:\n"),
    );
    expect(eligibility).toContain("expected_repository='NongHua123/fyagent'");
    expect(eligibility).toContain("expected_repository_id='1313497021'");
    expect(eligibility).toContain("GITHUB_WORKFLOW_REF");
    expect(eligibility).toContain("GITHUB_WORKFLOW_SHA");
    expect(eligibility).toContain(
      "Preflight must run from trusted refs/heads/main",
    );
    expect(eligibility).toContain(
      "v0.3.0 preflight must attest the exact trusted main workflow commit",
    );
    expect(eligibility).toContain("path: candidate-source");
    expect(eligibility).toContain(
      'cp scripts/version.mjs "$candidate_contract_root/scripts/version.mjs"',
    );
    expect(eligibility).not.toContain("pnpm install");
    expect(eligibility).toContain(
      "if: steps.contract.outputs.release_mode == 'formal'",
    );
    expect(eligibility).toContain("refs/tags/v0.3.0");
    expect(eligibility).toContain("git merge-base --is-ancestor");
    expect(eligibility).toContain("refs/remotes/origin/main");
    expect(eligibility).toContain("actions/workflows/ci.yml");
    expect(eligibility).toContain("branch=main");
    expect(eligibility).toContain("event=push");
    expect(eligibility).toContain("sort_by(.run_number, .run_attempt)");
    expect(eligibility).toContain('select(.name == "CI / Required")');
    expect(eligibility).toContain('.app.slug == "github-actions"');
    expect(eligibility).toContain(".url == $job_url");
    expect(eligibility).toContain(".head_sha == $sha");
    expect(eligibility).toContain("check-suites/${check_suite_id}/check-runs");
  });

  it("uses native Linux hosts with reviewed per-architecture Ubuntu 22.04 child digests", () => {
    expect(source).toContain("ubuntu:22.04@${{ matrix.container_digest }}");
    expect(source).toContain(
      "sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e",
    );
    expect(source).toContain(
      "sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149",
    );
    expect(source).toContain(
      "[ \"$(uname -m)\" = '${{ matrix.uname_arch }}' ]",
    );
    expect(source).toContain(
      "[ \"$RUNNER_ARCH\" = '${{ matrix.expected_runner_arch }}' ]",
    );
    expect(source).toContain("expected_runner_arch: X64");
    expect(source).toContain("expected_runner_arch: ARM64");
    expect(source.toLowerCase()).not.toContain("qemu");
    expect(source).toContain("Expected exactly one raw AppImage, DEB, and RPM");
    expect(source).toContain("dpkg-deb -f");
    expect(source).toContain("rpm -qp --qf");
  });

  it("preserves Windows elevation, helper, MSI table, payload, and unsigned gates", () => {
    expect(source.match(/FYAGENT_WINDOWS_MANIFEST: release/g)).toHaveLength(2);
    expect(source).toContain("--package fyagent-installer-actions");
    expect(source).toContain("0xAA64");
    expect(source).toContain("0x8664");
    expect(source).toContain("verify-windows-release-manifest.ps1");
    expect(source).toContain("-Architecture '${{ matrix.architecture }}'");
    expect(source).toContain("verify-windows-msi-structure.ps1");
    expect(source).toContain("verify-windows-msi.ps1");
    expect(source).toContain("verify-windows-unsigned.ps1");
    expect(source).not.toContain("${{ secrets.");
    expect(source).not.toContain("signtool.exe sign");

    expect(windowsManifestVerifier).toContain("mt.exe");
    expect(windowsManifestVerifier).toContain("RT_MANIFEST");
    expect(windowsManifestVerifier).toContain("requireAdministrator");
    expect(windowsManifestVerifier).toContain("0xAA64");
    expect(windowsManifestVerifier).toContain("0x8664");
    expect(windowsMsiStructureVerifier).toContain("WindowsInstaller.Installer");
    expect(windowsMsiStructureVerifier).toContain(
      "ValidateFyAgentInstallDirUi",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "ValidateFyAgentInstallDirExecute",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "AbortUnsafeFyAgentInstallDir",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "AbortUntrustedFyAgentMaintenance",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "MSI INSTALLDIR component guard drifted",
    );
    expect(windowsMsiStructureVerifier).toContain("MSI sequence order failed");
    expect(windowsMsiStructureVerifier).toContain(
      "SELECT ``Data`` FROM ``_Streams``",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "& expand.exe $cabinetPath '-F:Path'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "Extracted MSI fyagent.exe SHA-256 differs",
    );
    expect(windowsMsiVerifier).toContain("Binary.FyAgentInstallerActions");
    expect(windowsMsiVerifier).toContain(
      "complete fyagent URL protocol registry contract",
    );
    expect(windowsMsiVerifier).toContain(
      "Linux or retired cross-build host-path residue",
    );
    expect(windowsUnsignedVerifier).toContain("SignatureStatus]::NotSigned");
    expect(windowsUnsignedVerifier).toContain("TimeStamperCertificate");
  });

  it("builds one unsigned universal macOS app and rejects distribution trust", () => {
    expect(source).toContain("--target universal-apple-darwin --bundles app");
    expect(source).toContain("lipo -archs");
    expect(source).toContain("CFBundleShortVersionString");
    expect(source).toContain("CFBundleIdentifier");
    expect(source).toContain("com.fyagent.desktop");
    expect(source).toContain("TeamIdentifier");
    expect(source).toContain("Developer ID");
    expect(source).toContain("xcrun stapler validate");
    expect(source).not.toContain("stapler staple");
    expect(source).not.toContain("notarytool");
    expect(source).toContain("hdiutil attach");
    expect(source).toContain("-readonly");
    expect(source).toContain("shasum -a 256");
  });

  it("attests exactly ten installers plus manifest and metadata, then adds one bundle", () => {
    expect(source).toContain(
      "actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6",
    );
    expect(source).toContain("subject-path: verified-subjects/*");
    expect(source).toContain("verify-release-files.mjs installers installers");
    expect(source).toContain("collect-workflow-artifacts.mjs");
    expect(source).toContain("installers downloaded-installers installers");
    expect(source).toContain("metadata downloaded-metadata platform-metadata");
    expect(source).not.toContain("merge-multiple: true");
    expect(source).toContain(
      "verify-release-files.mjs subjects verified-subjects",
    );
    expect(source).toContain(
      "verify-release-files.mjs attachments release-attachments",
    );
    expect(source).toContain("download-manifest.json");
    expect(source).toContain("build-metadata.json");
    expect(source).toContain("artifact-attestation.sigstore.json");
    const verifyJob = source.slice(
      source.indexOf("\n  verify-assets:\n"),
      source.indexOf("\n  attest:\n"),
    );
    const attestJob = source.slice(
      source.indexOf("\n  attest:\n"),
      source.indexOf("\n  publish:\n"),
    );
    expect(verifyJob).toContain(
      "ref: ${{ needs.eligibility.outputs.workflow_sha }}",
    );
    expect(attestJob).toContain(
      "ref: ${{ needs.eligibility.outputs.workflow_sha }}",
    );
    expect(attestJob).toContain('[ -s "$BUNDLE_PATH" ]');
  });

  it("publishes once through a verified private draft and never auto-deletes failure residue", () => {
    const publish = source.slice(source.indexOf("\n  publish:\n"));
    expect(publish).toContain("releases?per_page=100");
    expect(publish).toContain("draft:true,prerelease:false");
    expect(publish).toContain('all(.state == "uploaded" and .size > 0)');
    expect(publish).toContain("Re-downloaded bytes differ");
    expect(publish).toContain("draft:false,prerelease:false");
    expect(publish).toContain('make_latest:"true"');
    expect(publish).toContain("releases/latest");
    expect(publish).toContain("publish_attempted=false");
    expect(publish).toContain("publish_attempted=true");
    expect(publish).toContain("failure-release-state.json");
    expect(publish).toContain('if .draft then "draft" else "published" end');
    expect(publish).toMatch(/case "\$observed_state" in\s+draft\)/);
    expect(publish).toMatch(/draft\)[\s\S]+published\)[\s\S]+\*\)/);
    expect(publish).toContain("The publish outcome is unknown");
    expect(publish).toContain("published-confirmed.json");
    expect(publish.indexOf("publish_attempted=true")).toBeLessThan(
      publish.indexOf("--request PATCH"),
    );
    expect(publish.lastIndexOf("draft_created=false")).toBeGreaterThan(
      publish.indexOf("releases/latest"),
    );
    expect(publish).not.toContain("failed after creating private draft");
    expect(publish).toContain("docs/release-notes/v0.3.0-en.md");
    expect(publish).not.toContain("gh release create");
    expect(publish).not.toContain("--request DELETE");
    expect(publish).not.toContain("gh release delete");
    expect(publish).not.toMatch(/git (?:push --delete|tag -d)/);
  });

  it("keeps manual installation assets free of an updater chain", () => {
    const normalizedSource = source.toLowerCase();
    expect(source).not.toMatch(
      /(?:verified-subjects|release-attachments)\/latest\.json/i,
    );
    expect(normalizedSource).not.toContain("tauri_signing_private_key");
    expect(normalizedSource).not.toContain("portable");
    expect(source).toContain("FyAgent-${APP_VERSION}-macOS.dmg");
    expect(source).toContain("FyAgent-$env:APP_VERSION-Windows");
    expect(source).toContain(
      "FyAgent-${APP_VERSION}-Linux-${{ matrix.asset_arch }}.AppImage",
    );
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
  const windowsMsiStructureVerifier = fs.readFileSync(
    WINDOWS_MSI_STRUCTURE_VERIFIER,
    "utf8",
  );

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
    expect(windowsMsiStructureVerifier).toContain("WindowsInstaller.Installer");
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
    expect(windowsMsiStructureVerifier).toContain("MsiLockPermissionsEx");
    expect(windowsMsiStructureVerifier).toContain("Test-InstallDirDescendant");
    expect(windowsMsiStructureVerifier).toContain("Assert-MsiCustomAction");
    expect(windowsMsiStructureVerifier).toContain(
      "ValidateFyAgentInstallDirExecute",
    );
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
