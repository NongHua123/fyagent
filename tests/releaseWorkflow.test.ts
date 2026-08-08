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
const INSTALLER_ACTIONS_MSI = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "installer-actions",
  "src",
  "msi.rs",
);
const INSTALLER_ACTIONS_COMPONENT_CLOSURE = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "installer-actions",
  "src",
  "component_closure.rs",
);
const INSTALLER_ACTIONS_MSI_PROBE = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "installer-actions",
  "src",
  "msi_probe.rs",
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

function workflowJobBlock(source: string, job: string, nextJob: string) {
  const start = source.indexOf(`\n  ${job}:\n`);
  const end = source.indexOf(`\n  ${nextJob}:\n`);
  expect(start).toBeGreaterThanOrEqual(0);
  expect(end).toBeGreaterThan(start);
  return source.slice(start, end);
}

function namedStepBlock(source: string, name: string) {
  const start = source.indexOf(`\n      - name: ${name}\n`);
  expect(start).toBeGreaterThanOrEqual(0);
  const end = source.indexOf("\n      - name:", start + 1);
  return source.slice(start, end < 0 ? source.length : end);
}

function expectExactLine(source: string, line: string) {
  expect(
    source.split(/\r?\n/).filter((candidate) => candidate === line),
  ).toEqual([line]);
}

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

  it("keeps authorized run observation synchronous and completion-scoped", () => {
    expect(source).toContain(
      "Authorized callers wait synchronously for this whole run to complete",
    );
    expect(source).toContain("read its final state once");
    expect(source).toContain(
      "fetch failed-job logs only after the completed run reports failure",
    );
    for (const forbiddenMonitor of [
      "gh run watch",
      "gh run view",
      "Start-Job",
      "Start-ThreadJob",
      "Start-Process",
      "nohup",
      "disown",
    ]) {
      expect(source).not.toContain(forbiddenMonitor);
    }
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
    expect(source).not.toContain("actions/cache");
    expect(source).not.toContain("cache: true");
    expect(source).not.toContain("cache: pnpm");
    expect(source.match(/uses: actions\/checkout@/g)).toHaveLength(
      source.match(/persist-credentials: false/g)?.length ?? 0,
    );
  });

  it("bootstraps native jobs without implicit tools, broad Git trust, or caches", () => {
    const nativeJobs = [
      {
        block: workflowJobBlock(source, "build-windows", "build-linux"),
        rustStep: "Setup Rust",
      },
      {
        block: workflowJobBlock(source, "build-linux", "build-macos"),
        rustStep: "Setup Rust",
      },
      {
        block: workflowJobBlock(source, "build-macos", "verify-assets"),
        rustStep: "Setup Rust with both universal targets",
      },
    ];

    for (const { block, rustStep } of nativeJobs) {
      const nodeIndex = block.indexOf("- name: Setup Node.js");
      const pnpmIndex = block.indexOf("- name: Setup pnpm");
      expect(nodeIndex).toBeGreaterThanOrEqual(0);
      expect(pnpmIndex).toBeGreaterThan(nodeIndex);
      const nodeStep = namedStepBlock(block, "Setup Node.js");
      const pnpmStep = namedStepBlock(block, "Setup pnpm");
      const rustSetupStep = namedStepBlock(block, rustStep);
      expect(nodeStep).toContain("uses: actions/setup-node@");
      expect(nodeStep).toContain("          node-version-file: .node-version");
      expect(pnpmStep).toContain("uses: pnpm/action-setup@");
      expectExactLine(pnpmStep, "          run_install: false");
      expectExactLine(pnpmStep, "          cache: false");
      expect(rustSetupStep).toContain(
        "uses: actions-rust-lang/setup-rust-toolchain@",
      );
      expectExactLine(rustSetupStep, "          cache: false");
    }

    const linux = nativeJobs[1].block;
    const checkoutIndex = linux.indexOf("- name: Checkout immutable source");
    const trustIndex = linux.indexOf(
      "- name: Trust exact checked-out workspace for container Git",
    );
    const nodeIndex = linux.indexOf("- name: Setup Node.js");
    expect(checkoutIndex).toBeGreaterThanOrEqual(0);
    expect(trustIndex).toBeGreaterThan(checkoutIndex);
    expect(nodeIndex).toBeGreaterThan(trustIndex);
    const trustStep = namedStepBlock(
      linux,
      "Trust exact checked-out workspace for container Git",
    );
    expectExactLine(
      trustStep,
      "          git config --global --unset-all safe.directory 2>/dev/null || true",
    );
    expectExactLine(
      trustStep,
      '          git config --global --add safe.directory "$GITHUB_WORKSPACE"',
    );
    expectExactLine(
      trustStep,
      '          [ "$(git config --get-all safe.directory)" = "$GITHUB_WORKSPACE" ] || {',
    );
    expectExactLine(
      trustStep,
      '          [ "$(git -C "$GITHUB_WORKSPACE" rev-parse HEAD)" = "$SOURCE_SHA" ]',
    );
    expect(trustStep.match(/safe\.directory/g)).toHaveLength(3);
    expect(source).not.toMatch(/safe\.directory\s+["']?\*["']?/);
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
    const linuxJob = workflowJobBlock(source, "build-linux", "build-macos");
    const packageStep = namedStepBlock(linuxJob, "Build native Linux packages");
    expectExactLine(packageStep, '          APPIMAGE_EXTRACT_AND_RUN: "1"');
    expectExactLine(
      packageStep,
      "        run: pnpm tauri build --bundles appimage,deb,rpm --verbose",
    );
    expect(source.match(/APPIMAGE_EXTRACT_AND_RUN:/g)).toHaveLength(1);
    expect(packageStep).not.toContain("privileged:");
    expect(source).not.toContain("SYS_ADMIN");
    expect(source).not.toContain("/dev/fuse");
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
    expect(source).not.toContain("-sval");

    const windowsJob = workflowJobBlock(source, "build-windows", "build-linux");
    const bundleStep = namedStepBlock(
      windowsJob,
      "Bundle unsigned Windows MSI",
    );
    const bundleLines = bundleStep.split(/\r?\n/);
    const bundleCommandIndexes = bundleLines
      .map((line, index) => ({ line: line.trim(), index }))
      .filter(({ line }) => line.startsWith("pnpm tauri bundle"));
    expect(bundleCommandIndexes.map(({ line }) => line)).toEqual([
      "pnpm tauri bundle --target '${{ matrix.rust_target }}' --bundles msi --verbose",
      "pnpm tauri bundle --bundles msi --verbose",
    ]);
    for (const { index } of bundleCommandIndexes) {
      expect(bundleLines[index + 1]?.trim()).toBe(
        "$bundleExitCode = $LASTEXITCODE",
      );
    }
    expect(bundleStep).toContain("if ($bundleExitCode -ne 0)");
    expect(bundleStep.indexOf("if ($bundleExitCode -ne 0)")).toBeLessThan(
      bundleStep.indexOf("Get-ChildItem"),
    );

    expect(windowsManifestVerifier).toContain("Resolve-WindowsSdkManifestTool");
    expect(windowsManifestVerifier).toContain("ProgramFiles(x86)");
    expect(windowsManifestVerifier).toContain("Windows Kits\\10\\bin");
    expect(windowsManifestVerifier).toContain("[Version]::TryParse");
    expect(windowsManifestVerifier).toContain(
      "$sdkArchitecture = if ($TargetArchitecture -eq 'arm64') { 'arm64' } else { 'x64' }",
    );
    expect(windowsManifestVerifier).toContain('"$sdkArchitecture\\mt.exe"');
    expect(windowsManifestVerifier).toContain(
      "Sort-Object -Property @{ Expression = 'Version'; Descending = $true }",
    );
    expect(windowsManifestVerifier).toContain("Select-Object -First 1");
    expect(windowsManifestVerifier).toContain(
      'throw "Architecture-matched Windows SDK mt.exe was not found for $TargetArchitecture"',
    );
    expect(windowsManifestVerifier).toContain(
      "$mtPath = Resolve-WindowsSdkManifestTool -TargetArchitecture $Architecture",
    );
    expect(windowsManifestVerifier).toContain(
      '& $mtPath "-inputresource:$resolvedExe;#1" "-out:$manifestPath" -nologo',
    );
    expect(windowsManifestVerifier).not.toContain("Get-Command mt.exe");
    expect(windowsManifestVerifier).not.toContain("& mt.exe");
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
      "ClassifyFyAgentPureUninstall",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "private pure-uninstall classifier marker",
    );
    expect(windowsMsiStructureVerifier).toContain("required core component");
    expect(windowsMsiStructureVerifier).toContain(
      "context-redirected DesktopFolder",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "context-redirected ProgramMenuFolder",
    );
    expect(windowsMsiStructureVerifier).toContain("MSI sequence order failed");
    expect(windowsMsiStructureVerifier).toContain(
      "SELECT ``Data`` FROM ``_Streams``",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "Start-Process -FilePath $expandCommand.Source",
    );
    expect(windowsMsiStructureVerifier).toContain("'-F:Path'");
    expect(windowsMsiStructureVerifier).toContain("$expandProcess.ExitCode");
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
  const installerActionsMsi = fs.readFileSync(INSTALLER_ACTIONS_MSI, "utf8");
  const installerActionsComponentClosure = fs.readFileSync(
    INSTALLER_ACTIONS_COMPONENT_CLOSURE,
    "utf8",
  );
  const installerActionsMsiProbe = fs.readFileSync(
    INSTALLER_ACTIONS_MSI_PROBE,
    "utf8",
  );
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
    expect(buildRs).toContain("cargo:rustc-link-arg=/MANIFEST:EMBED");
    expect(buildRs).toContain("cargo:rustc-link-arg=/MANIFESTINPUT:{}");
    expect(buildRs).toContain("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    expect(buildRs).not.toContain("cargo:rustc-link-arg-tests=");
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
    expect(source).toContain('<Directory Id="DesktopFolder" Name="Desktop" />');
    expect(source).toContain('<Directory Id="ProgramMenuFolder">');
    expect(source).toContain(
      '<Directory Id="ApplicationProgramsFolder" Name="{{product_name}}"/>',
    );
    expect(source).toContain(
      '<Property Id="DISABLEADVTSHORTCUTS" Value="1" />',
    );
    expectExactLine(
      source,
      '        <SetProperty Id="ALLUSERS" Action="EnforceFyAgentAllUsers" Value="1" Before="CostInitialize" Sequence="both">1</SetProperty>',
    );
    expectExactLine(
      source,
      '        <SetProperty Id="DISABLEADVTSHORTCUTS" Action="EnforceFyAgentDisableAdvertisedShortcuts" Value="1" Before="CostInitialize" Sequence="both">1</SetProperty>',
    );
    expect(source.match(/<Shortcut\b[^>]*\bAdvertise="yes"/g)).toHaveLength(2);
    expect(source).toContain('Directory="DesktopFolder"');
    expect(source).toContain('Directory="ApplicationProgramsFolder"');
    expect(source).not.toContain('Target="[!Path]"');
    expect(source).not.toContain('Icon="ProductIcon"');
    expect(source).not.toContain('Name="DesktopShortcut"');
    expect(source).not.toContain('Name="StartMenuShortcut"');
    expect(source).not.toContain('<Component Id="ApplicationShortcut"');
    expect(source).not.toContain('<Component Id="ApplicationShortcutDesktop"');
    expect(source).toContain(
      '<File Id="Path" Source="{{main_binary_path}}" KeyPath="yes" Checksum="yes">',
    );
    expect(source).toContain(
      'Name="PathComponent" Type="integer" Value="1" KeyPath="no"',
    );
    expect(source).not.toContain('<RemoveFolder Id="DesktopFolder"');
    expect(source).not.toContain('<RemoveFolder Id="ProgramMenuFolder"');
    expect(source).toContain(
      '<RemoveFolder Id="RemoveApplicationProgramsFolder" Directory="ApplicationProgramsFolder" On="uninstall"/>',
    );
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

  it("uses architecture-matched Type 1 validation and fail-closed uninstall classification", () => {
    expect(source).toContain(
      '<Binary Id="FyAgentInstallerActions" SourceFile="$(env.TAURI_FYAGENT_INSTALLER_ACTIONS_DLL)" />',
    );
    expect(source).toContain('Id="ValidateFyAgentInstallDirUi"');
    expect(source).toContain('DllEntry="ValidateFyAgentInstallDirUi"');
    expect(source).toContain('Id="ValidateFyAgentInstallDirExecute"');
    expect(source).toContain('DllEntry="ValidateFyAgentInstallDirExecute"');
    expect(source).toContain('Id="ClassifyFyAgentPureUninstall"');
    expect(source).toContain('DllEntry="ClassifyFyAgentPureUninstall"');
    expect(
      source.match(
        /<Custom Action="ClassifyFyAgentPureUninstall" After="CostFinalize">1<\/Custom>/g,
      ),
    ).toHaveLength(2);
    expect(source).toContain('Id="ApplyValidatedFyAgentInstallDir"');
    const applyActionStart = source.indexOf(
      '<CustomAction Id="ApplyValidatedFyAgentInstallDir"',
    );
    const applyActionEnd = source.indexOf("/>", applyActionStart);
    const applyAction = source.slice(applyActionStart, applyActionEnd + 2);
    expect(applyAction).toContain('Directory="INSTALLDIR"');
    expect(applyAction).not.toContain('Property="INSTALLDIR"');
    expect(source).toContain('Value="[FYAGENT_INSTALLDIR_NORMALIZED]"');
    expect(source).toContain('Id="AbortUnsafeFyAgentInstallDir"');
    expect(source).toContain('Error="[FYAGENT_INSTALLDIR_ERROR_MESSAGE]"');
    expect(source).toContain("ValidateFyAgentInstallDirUi");
    expect(source).toContain("ValidateFyAgentInstallDirExecute");
    expect(source).toContain("AbortUnsafeFyAgentInstallDir");
    expect(source).toContain("NOT FyAgentPureUninstall");
    expect(source).not.toContain("FYAGENT_PURE_UNINSTALL");
    expect(source).not.toContain("ClearFyAgentPureUninstall");
    expect(source).not.toContain("SetFyAgentPureUninstall");
    expect(source).not.toMatch(/\$[A-Za-z_][A-Za-z0-9_.]*\s*=\s*2/);
    expect(source).not.toContain('<Property Id="FyAgentPureUninstall"');
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
      "(Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE) AND NOT FYAGENT_PREVIOUS_INSTALLDIR AND NOT FyAgentPureUninstall",
    );
    expect(source).toContain(
      'Action="AbortUntrustedFyAgentMaintenance" After="ClassifyFyAgentPureUninstall"',
    );
    expect(source).toContain(
      'Action="ApplyValidatedFyAgentInstallDir" After="ValidateFyAgentInstallDirExecute">NOT Installed AND NOT WIX_UPGRADE_DETECTED AND NOT UPGRADINGPRODUCTCODE AND NOT FyAgentPureUninstall AND FYAGENT_INSTALLDIR_VALID = "1"',
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
    expect(installerActionsLib).toContain(
      "ClassifyFyAgentPureUninstall(install: MSIHANDLE)",
    );
    expect(installerActionsLib).toContain("session.clear_pure_uninstall()");
    expect(installerActionsLib).toContain(
      "session.install_dir_component_ids()",
    );
    expect(installerActionsLib).toContain(
      "session.components_all_absent(&components)",
    );
    expect(installerActionsMsi).toContain(
      'const PURE_UNINSTALL: &str = "FyAgentPureUninstall";',
    );
    expect(installerActionsMsi).toContain("MsiGetActiveDatabase");
    expect(installerActionsMsi).toContain("MsiDatabaseOpenViewW");
    expect(installerActionsMsi).toContain(
      "SELECT `Directory`, `Directory_Parent` FROM `Directory`",
    );
    expect(installerActionsMsi).toContain(
      "SELECT `Component`, `Directory_` FROM `Component`",
    );
    expect(installerActionsMsi).toContain("MsiGetComponentStateW");
    expect(installerActionsMsi).toContain("INSTALLSTATE_ABSENT");
    expect(installerActionsMsi).toContain("MsiViewClose");
    expect(installerActionsMsi).toContain("MAX_MSI_FIELD_UNITS");
    for (const coreComponent of [
      "CMP_UninstallShortcut",
      "InstallDirectoryAcl",
      "Path",
      "RegistryEntries",
    ]) {
      expect(installerActionsComponentClosure).toContain(`"${coreComponent}"`);
    }
    expect(installerActionsComponentClosure).toContain("DuplicateComponent");
    expect(installerActionsComponentClosure).toContain("MissingCoreComponent");
    expect(installerActionsLib).toContain("ERROR_SUCCESS");
    expect(installerActionsLib).toContain("ERROR_INSTALL_FAILURE");
  });

  it("uses writable zero-capacity probes for variable-length MSI strings", () => {
    expect(installerActionsLib).toContain("mod msi_probe;");
    const getPropertyStart = installerActionsMsi.indexOf(
      "pub(crate) fn get_property",
    );
    const getPropertyEnd = installerActionsMsi.indexOf(
      "pub(crate) fn validation_context",
      getPropertyStart,
    );
    const recordStringStart = installerActionsMsi.indexOf("fn record_string");
    const recordStringEnd = installerActionsMsi.indexOf(
      "fn wide_null",
      recordStringStart,
    );
    expect(getPropertyStart).toBeGreaterThanOrEqual(0);
    expect(getPropertyEnd).toBeGreaterThan(getPropertyStart);
    expect(recordStringStart).toBeGreaterThanOrEqual(0);
    expect(recordStringEnd).toBeGreaterThan(recordStringStart);

    const getProperty = installerActionsMsi.slice(
      getPropertyStart,
      getPropertyEnd,
    );
    const recordString = installerActionsMsi.slice(
      recordStringStart,
      recordStringEnd,
    );
    expect(installerActionsMsi.match(/let mut probe = 0_u16;/g)).toHaveLength(
      2,
    );
    expect(getProperty).toMatch(
      /MsiGetPropertyW\(\s*self\.handle,\s*name\.as_ptr\(\),\s*&mut probe,\s*&mut reported_length,?\s*\)/,
    );
    expect(recordString).toMatch(
      /MsiRecordGetStringW\(\s*record,\s*field,\s*&mut probe,\s*&mut reported_length\s*\)/,
    );
    for (const probeSource of [getProperty, recordString]) {
      expect(probeSource).toContain("let mut reported_length = 0_u32;");
      expect(probeSource).toContain(
        "msi_string_probe_disposition(first_status, reported_length)?",
      );
      expect(probeSource).not.toContain("std::ptr::null_mut()");
    }
    expect(installerActionsMsi).not.toContain("std::ptr::null_mut()");
    expect(
      installerActionsMsi.match(
        /msi_string_probe_disposition\(first_status, reported_length\)\?/g,
      ),
    ).toHaveLength(2);
    expect(installerActionsMsi).toContain(
      "ERROR_SUCCESS => MsiStringProbeStatus::Success",
    );
    expect(installerActionsMsi).toContain(
      "ERROR_MORE_DATA => MsiStringProbeStatus::MoreData",
    );
    expect(installerActionsMsiProbe).toContain(
      "(MsiStringProbeStatus::Success, 0) | (MsiStringProbeStatus::MoreData, 0)",
    );
    expect(installerActionsMsiProbe).toContain(
      "(MsiStringProbeStatus::MoreData, length)",
    );
    expect(installerActionsMsiProbe).toContain(
      "(MsiStringProbeStatus::Success, _) => Err(UnexpectedSuccessLength)",
    );
    expect(installerActionsMsiProbe).toContain(
      "accepts_both_documented_empty_probe_results",
    );
    expect(installerActionsMsiProbe).toContain(
      "requests_a_second_read_only_for_positive_more_data_lengths",
    );
    expect(installerActionsMsiProbe).toContain(
      "rejects_success_that_claims_unwritten_data",
    );
    expect(getProperty).toContain("*length <= 32_768");
    expect(recordString).toContain("reported_length > MAX_MSI_FIELD_UNITS");
    expect(recordString).toContain("length > reported_length");
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
    expect(installDirUi.match(/NOT FyAgentPureUninstall/g)).toHaveLength(4);
    expect(installDirUi).not.toContain("FYAGENT_PURE_UNINSTALL");
    expect(installDirUi).toContain(
      'NOT Installed AND NOT WIX_UPGRADE_DETECTED AND NOT UPGRADINGPRODUCTCODE AND FYAGENT_INSTALLDIR_VALID="1"',
    );
    expect(installDirUi).not.toMatch(/\$[A-Za-z_][A-Za-z0-9_.]*\s*=\s*2/);
    expect(releaseSource).toContain("FYAGENT_INSTALLER_ACTIONS_DLL");
    expect(releaseSource).toContain("TAURI_FYAGENT_INSTALLER_ACTIONS_DLL");
    expect(releaseSource).toContain("fyagent_installer_actions.dll");
    expect(windowsMsiStructureVerifier).toContain("MsiLockPermissionsEx");
    expect(windowsMsiStructureVerifier).toContain("Test-InstallDirDescendant");
    expect(windowsMsiStructureVerifier).toContain("Assert-MsiCustomAction");
    expect(windowsMsiStructureVerifier).toContain(
      "'ApplyValidatedFyAgentInstallDir' 35 'INSTALLDIR' '[FYAGENT_INSTALLDIR_NORMALIZED]'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "'ClassifyFyAgentPureUninstall' 1 'FyAgentInstallerActions' 'ClassifyFyAgentPureUninstall'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "'EnforceFyAgentAllUsers' 51 'ALLUSERS' '1'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "'EnforceFyAgentDisableAdvertisedShortcuts' 51 'DISABLEADVTSHORTCUTS' '1'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "Assert-MsiSequenceCondition $table 'EnforceFyAgentAllUsers' '1'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "Assert-MsiSequenceCondition $table 'EnforceFyAgentDisableAdvertisedShortcuts' '1'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "enforce ALLUSERS before CostInitialize",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "enforce DISABLEADVTSHORTCUTS before CostInitialize",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "requires exactly one sequence action",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "Assert-MsiPropertyValue 'ALLUSERS' '1'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "Assert-MsiPropertyValue 'DISABLEADVTSHORTCUTS' '1'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "$targetFeature -cnotmatch '^[A-Za-z_][A-Za-z0-9_.]{0,37}$'",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "CostFinalize before native pure-uninstall classifier",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "must not author a default for the private pure-uninstall classifier marker",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "condition exceeds the 255-character table limit",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "ControlEvent condition exceeds the 255-character table limit",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "'NOT Installed AND NOT WIX_UPGRADE_DETECTED AND NOT UPGRADINGPRODUCTCODE'",
    );
    expect(windowsMsiStructureVerifier).toContain("advertised feature target");
    expect(windowsMsiStructureVerifier).toContain("FeatureComponents");
    expect(windowsMsiStructureVerifier).toContain(
      "desktop and Start Menu shortcuts must target the same Path-owning feature",
    );
    expect(windowsMsiStructureVerifier).toContain("DISABLEADVTSHORTCUTS");
    expect(windowsMsiStructureVerifier).toContain(
      "installed executable as its file KeyPath",
    );
    expect(windowsMsiStructureVerifier).toContain("[StringComparer]::Ordinal");
    expect(windowsMsiStructureVerifier).toContain(
      "InstallExecuteSequence RemoveShortcuts before RemoveFiles",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "MSI product Start Menu directory cleanup row drifted",
    );
    expect(windowsMsiStructureVerifier).toContain("$maxDirectoryRows = 4096");
    expect(windowsMsiStructureVerifier).toContain("$maxComponentRows = 32768");
    expect(windowsMsiStructureVerifier).toContain(
      "$maxMsiFieldUtf16Units = 1024",
    );
    expect(windowsMsiStructureVerifier).toContain("-MaxRows $maxDirectoryRows");
    expect(windowsMsiStructureVerifier).toContain("-MaxRows $maxComponentRows");
    expect(windowsMsiStructureVerifier).toContain(
      "$value.Length -gt $maxMsiFieldUtf16Units",
    );
    expect(windowsMsiStructureVerifier).toContain(
      "[void]$rows.Add([PSCustomObject]@{ Values = $values })",
    );
    expect(windowsMsiStructureVerifier).toContain("return $rows.ToArray()");
    expect(
      windowsMsiStructureVerifier.match(/Release-ComObject \$record/g),
    ).toHaveLength(2);
    expect(
      windowsMsiStructureVerifier.match(/Release-ComObject \$view/g),
    ).toHaveLength(1);
    expect(windowsMsiStructureVerifier).toContain(
      "FinalReleaseComObject($Value)",
    );
    expect(
      windowsMsiStructureVerifier.match(/\$record\.StringData\(/g),
    ).toHaveLength(1);
    expect(windowsMsiStructureVerifier).not.toContain(".IntegerData(");
    expect(
      windowsMsiStructureVerifier.match(/\[void\]\$view\.Execute\(\)/g),
    ).toHaveLength(2);
    expect(
      windowsMsiStructureVerifier.match(/\[void\]\$view\.Close\(\)/g),
    ).toHaveLength(2);
    expect(windowsMsiStructureVerifier).not.toMatch(/^\s*\$view\.Execute\(\)/m);
    expect(
      windowsMsiVerifier.match(/\[void\]\$(?:view|binaryView)\.Execute\(\)/g),
    ).toHaveLength(2);
    expect(
      windowsMsiVerifier.match(/\[void\]\$(?:view|binaryView)\.Close\(\)/g),
    ).toHaveLength(2);
    expect(
      windowsMsiVerifier.match(/\[void\]\$[A-Za-z]+\.Add\(/g),
    ).toHaveLength(2);
    expect(windowsMsiVerifier).not.toMatch(
      /^\s*\$(?:view|binaryView)\.Execute\(\)/m,
    );
    expect(windowsMsiVerifier).not.toMatch(/^\s*\$[A-Za-z]+\.Add\(/m);
    expect(windowsMsiVerifier).not.toMatch(
      /^\s*\$(?:view|binaryView)\.Close\(\)/m,
    );
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
