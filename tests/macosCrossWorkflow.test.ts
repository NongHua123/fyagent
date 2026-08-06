import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const SCRIPT_DIR = path.join(ROOT, "scripts", "macos-cross");
const read = (relative: string) =>
  fs.readFileSync(path.join(ROOT, relative), "utf8").replace(/\r\n/g, "\n");

describe("WSL macOS Universal DMG workflow", () => {
  const constants = read("scripts/macos-cross/constants.sh");
  const publicEntrypoint = read("scripts/macos-cross/build-universal-dmg.sh");
  const buildPipeline = read("scripts/macos-cross/build-package.sh");
  const windowsCrossBuild = read("scripts/windows-cross/build-windows-msi.sh");
  const verifier = read("scripts/macos-cross/verify_artifacts.py");
  const bootstrap = read("scripts/macos-cross/bootstrap-host.sh");
  const provision = read("scripts/macos-cross/provision-toolchains.sh");
  const mise = read("mise.toml");
  const gitignore = read(".gitignore");
  const workflowSource = fs
    .readdirSync(SCRIPT_DIR)
    .filter((file) => file.endsWith(".sh") || file.endsWith(".py"))
    .map((file) => fs.readFileSync(path.join(SCRIPT_DIR, file), "utf8"))
    .join("\n");

  it("pins every downloaded input instead of following moving refs", () => {
    expect(constants).toContain(
      'FYAGENT_MACOS_SDK_SHA256="6e146275d19f027faa2e8354da5e0267513abf013b8f16ad65a231653a2b1c5d"',
    );
    expect(constants).toContain(
      'FYAGENT_OSXCROSS_COMMIT="27d21e4977c9751d01199c7a226a6faf494c3dd9"',
    );
    expect(constants).toContain(
      'FYAGENT_LIBDMG_COMMIT="7ac55ec64c96f7800d9818ce64c79670e7f02b67"',
    );
    expect(constants).toContain('FYAGENT_RCODESIGN_VERSION="0.29.0"');
    expect(constants).toContain(
      'FYAGENT_RCODESIGN_SHA256="dbe85cedd8ee4217b64e9a0e4c2aef92ab8bcaaa41f20bde99781ff02e600002"',
    );
    expect(constants).not.toMatch(/=(?:"|')(?:master|main|latest)(?:"|')/);
  });

  it("uses the user-installed global mise with repository-pinned tools", () => {
    expect(constants).toContain('FYAGENT_MISE_MIN_VERSION="2026.8.0"');
    expect(publicEntrypoint).toContain("command -v mise");
    expect(publicEntrypoint).toContain(
      "global mise is required; install mise and rerun",
    );
    expect(publicEntrypoint).toContain(
      "global mise resolves to a Windows-mounted path",
    );
    expect(bootstrap).toContain("global mise accepted");
    expect(provision).toContain(
      '"$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" install',
    );
    for (const forbidden of [
      "FYAGENT_MISE_URL",
      "FYAGENT_MISE_SHA256",
      "MISE_DATA_DIR=",
      "MISE_CACHE_DIR=",
      "MISE_STATE_DIR=",
      "MISE_CARGO_HOME=",
      "MISE_RUSTUP_HOME=",
      "CARGO_HOME=",
      "RUSTUP_HOME=",
      "workflow-private mise",
    ]) {
      expect(workflowSource).not.toContain(forbidden);
    }
  });

  it("fails closed on unsupported hosts and unaccepted fixed-source risk", () => {
    expect(bootstrap).toContain("this workflow requires WSL2");
    expect(bootstrap).toContain("only x86_64 WSL hosts are supported");
    expect(bootstrap).toContain("only Ubuntu 22.04 and 24.04 are supported");
    expect(bootstrap).toContain(
      "DrvFS paths under /mnt/<drive> are unsupported",
    );
    expect(bootstrap).toContain(
      "risk acknowledgement is required; rerun with --accept-risk",
    );
  });

  it("keeps the public interface Universal-only", () => {
    expect(publicEntrypoint).toContain(
      "build-universal-dmg.sh [--accept-risk]",
    );
    expect(publicEntrypoint).not.toContain("--target");
    expect(publicEntrypoint).not.toContain("aarch64-apple-darwin");
    expect(publicEntrypoint).not.toContain("x86_64-apple-darwin");
    expect(buildPipeline).toContain(
      "pnpm tauri build --target universal-apple-darwin --no-bundle --ci",
    );
    expect(buildPipeline).toContain("CI=true pnpm install --frozen-lockfile");
  });

  it("exposes the supported cross-build tasks and publication roots", () => {
    for (const [task, command] of [
      [
        "build:cross-windows:x64",
        "bash scripts/windows-cross/build-windows-msi.sh --arch x64",
      ],
      [
        "build:cross-windows:arm64",
        "bash scripts/windows-cross/build-windows-msi.sh --arch arm64",
      ],
      [
        "build:cross-windows",
        "bash scripts/windows-cross/build-windows-msi.sh --arch all",
      ],
      [
        "build:cross-macos:universal",
        "bash scripts/macos-cross/build-universal-dmg.sh",
      ],
    ]) {
      expect(mise).toContain(`[tasks."${task}"]`);
      expect(mise).toContain(`run = "${command}"`);
    }
    expect(mise).toContain(
      '[tasks."build:cross-macos:universal"]\ndescription = "Build the experimental macOS Universal DMG cross-build artifact (prompts for risk acknowledgement)"\ninteractive = true\nraw_args = true\nrun = "bash scripts/macos-cross/build-universal-dmg.sh"',
    );
    expect(mise).toContain("task.run_auto_install = false");

    expect(windowsCrossBuild).toContain(
      'OUTPUT_ROOT="${FYAGENT_WINDOWS_OUTPUT_ROOT:-$PROJECT_ROOT/dist-bundle/windows}"',
    );
    expect(buildPipeline).toContain(
      'dist_dir="$PROJECT_ROOT/dist-bundle/macos"',
    );
    expect(gitignore).toContain("/dist-bundle/");
  });

  it("signs the app before packaging and the DMG before checksumming", () => {
    const appSign = buildPipeline.indexOf('rcodesign sign "$app_path"');
    const packageDmg = buildPipeline.indexOf('"$SCRIPT_DIR/make-dmg.sh"');
    const dmgSign = buildPipeline.indexOf('rcodesign sign "$temporary_dmg"');
    const checksum = buildPipeline.indexOf('sha256sum "$artifact_base"');
    expect(appSign).toBeGreaterThan(-1);
    expect(packageDmg).toBeGreaterThan(appSign);
    expect(dmgSign).toBeGreaterThan(packageDmg);
    expect(checksum).toBeGreaterThan(dmgSign);
  });

  it("labels the only installer artifact with every trust boundary", () => {
    expect(buildPipeline).toContain(
      "macOS-universal-adhoc-unnotarized-experimental.dmg",
    );
    expect(buildPipeline).not.toContain(".zip");
    expect(verifier).toContain('"macosNativeValidation": "pending"');
    expect(verifier).toContain('"worktreePolicy": "unchecked"');
    expect(verifier).toContain('"wslVersion": 2');
    expect(verifier).toContain('"fileVault": "disabled"');
    expect(verifier).toContain("CodeSignatureFlags(ADHOC)");
    expect(buildPipeline).toContain("manifest-check");
  });

  it("keeps host-built cross tools in keyed FyAgent storage", () => {
    expect(constants).toContain("FYAGENT_HOST_CACHE_KEY");
    expect(constants).toContain('FYAGENT_LIBDMG_FILEVAULT_MODE="disabled"');
    expect(provision).toContain("libdmg unexpectedly linked libcrypto");
    expect(provision).toContain("cache hit: OSXCross");
    expect(provision).toContain("cache hit: libdmg-hfsplus");
    expect(provision).toContain("cache hit: rcodesign");
  });

  it("does not consume Apple release credentials", () => {
    for (const secret of [
      "APPLE_CERTIFICATE",
      "APPLE_ID",
      "APPLE_PASSWORD",
      "APPLE_TEAM_ID",
      "APPLE_API_KEY",
      "NOTARIZE",
    ]) {
      expect(workflowSource).not.toContain(secret);
    }
  });

  it("keeps mise synchronized with existing project runtime declarations", () => {
    const packageJson = JSON.parse(read("package.json")) as {
      packageManager: string;
    };
    expect(mise).toContain('min_version = "2026.8.0"');
    expect(mise).toContain(`node = "${read(".node-version").trim()}"`);
    expect(mise).toContain(
      `pnpm = "${packageJson.packageManager.split("@")[1]}"`,
    );
    expect(mise).toContain('python = "3.12.8"');
    expect(mise).toContain('version = "1.95.0"');
    expect(mise).toContain('components = ["rustfmt", "clippy", "llvm-tools"]');
    for (const target of [
      "aarch64-apple-darwin",
      "x86_64-apple-darwin",
      "aarch64-pc-windows-msvc",
      "x86_64-pc-windows-msvc",
    ]) {
      expect(mise).toContain(`"${target}"`);
    }

    const lock = read("mise.lock");
    for (const platform of [
      "linux-x64",
      "linux-arm64",
      "macos-x64",
      "macos-arm64",
      "windows-x64",
      "windows-arm64",
    ]) {
      expect(mise).toContain(`"${platform}"`);
      expect(lock).toContain(`platforms.${platform}`);
    }
    expect(lock).toContain('version = "22.12.0"');
    expect(lock).toContain('version = "10.12.3"');
    expect(lock).toContain('version = "3.12.8"');
    expect(lock).toContain('version = "1.95.0"');
    expect(lock).toContain('components = "clippy,llvm-tools,rustfmt"');
    expect(lock).toContain(
      'targets = "aarch64-apple-darwin,aarch64-pc-windows-msvc,x86_64-apple-darwin,x86_64-pc-windows-msvc"',
    );
  });

  it("documents mise as the default local development version manager", () => {
    const spec = read(".trellis/spec/backend/development-environment.md");
    expect(spec).toContain(
      "global mise is the required version manager and command",
    );
    expect(spec).toContain("local development on");
    expect(spec).toContain("mise exec -- <command>");
    expect(spec).toContain(
      "mise lock --platform linux-x64,linux-arm64,macos-x64,macos-arm64,windows-x64,windows-arm64",
    );

    const developmentDocuments = [
      "README.md",
      "README_ZH.md",
      "README_JA.md",
      "README_DE.md",
      "CONTRIBUTING.md",
    ];
    for (const document of developmentDocuments) {
      const content = read(document);
      expect(content).toContain("https://mise.jdx.dev/getting-started.html");
      expect(content).toContain("mise install");
      expect(content).toContain("mise exec -- pnpm");
      expect(content).not.toMatch(/Node\.js (?:18|20)\+/);
      expect(content).not.toContain("pnpm 8+");
      expect(content).not.toContain("Rust 1.85+");
    }

    for (const document of [
      "README.md",
      "README_ZH.md",
      "README_JA.md",
      "README_DE.md",
    ]) {
      const content = read(document);
      expect(content).toMatch(/^mise run build:cross-windows:x64$/m);
      expect(content).toMatch(/^mise run build:cross-windows:arm64$/m);
      expect(content).toMatch(/^mise run build:cross-windows$/m);
      expect(content).toMatch(/^mise run build:cross-macos:universal$/m);
    }
  });
});
