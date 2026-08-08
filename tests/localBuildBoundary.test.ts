import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const read = (relative: string) =>
  fs.readFileSync(path.join(ROOT, relative), "utf8").replace(/\r\n/g, "\n");

type CommandPlan = {
  command: string;
  args: string[];
  target: string;
  environment: Record<string, string>;
};
type HostNativeModule = {
  HOST_RUST_TARGETS: Readonly<Record<string, string>>;
  expectedRustTarget(platform: string, architecture: string): string;
  parseRustcHost(verboseVersion: string): string;
  assertCurrentRustHost(input: {
    platform: string;
    architecture: string;
    rustcVerboseVersion: string;
  }): string;
  containsRustTargetOverride(value: string, encoded?: boolean): boolean;
  assertNoCallerTargetOverride(environment: Record<string, string>): void;
  buildNativeRunnerConfig(input: {
    target: string;
    platform: string;
    nodeExecutable: string;
    runnerScript: string;
  }): string;
  planTauriTask(input: {
    operation: string;
    forwardedArguments?: string[];
    environment: Record<string, string>;
    platform: string;
    architecture: string;
    rustcVerboseVersion: string;
    rustdocVerboseVersion: string;
    rustcExecutable: string;
    rustdocExecutable: string;
  }): CommandPlan;
  planCargoTask(input: {
    operation: string;
    filters?: string[];
    forwardedArguments?: string[];
    environment: Record<string, string>;
    platform: string;
    architecture: string;
    rustcVerboseVersion: string;
    rustdocVerboseVersion: string;
    rustcExecutable: string;
    rustdocExecutable: string;
    nativeRunnerConfig: string;
  }): CommandPlan;
};

let hostNative: HostNativeModule;

beforeAll(async () => {
  hostNative = (await import(
    /* @vite-ignore */ pathToFileURL(
      path.join(ROOT, "scripts", "tasks", "host-native.mjs"),
    ).href
  )) as HostNativeModule;
});

const RETIRED_TASKS = [
  "macos:preflight",
  "build:cross-windows:x64",
  "build:cross-windows:arm64",
  "build:cross-windows",
  "build:cross-macos:universal",
];
const RETIRED_PATHS = ["scripts/macos-cross", "scripts/windows-cross"];
const LOCAL_CROSS_EXECUTION_MARKERS = [
  "--target",
  "CARGO_BUILD_TARGET",
  "TAURI_ENV_TARGET_TRIPLE",
  "cargo-xwin",
  "cargo-zigbuild",
  "cross build",
  "osxcross",
  "qemu",
  "wine",
  "wsl.exe",
  "scripts/macos-cross",
  "scripts/windows-cross",
  "src-tauri/target/app",
  "target/installer-actions",
  "universal-apple-darwin",
  "pc-windows-msvc",
  "rustup target add",
  "wixl",
];
const CURRENT_DOCUMENTS = [
  "README.md",
  "README_ZH.md",
  "README_JA.md",
  "README_DE.md",
  "CONTRIBUTING.md",
  ".trellis/spec/backend/index.md",
  ".trellis/spec/backend/development-environment.md",
  ".trellis/spec/backend/fyagent-version-contract.md",
  ".trellis/spec/backend/windows-release-boundary.md",
];

function executableRepositoryFiles(): string[] {
  const roots = [
    "mise.toml",
    "mise.lock",
    "scripts",
    ".github/workflows",
    ".trellis/scripts",
    ".mise",
    ".codex",
  ];
  const files: string[] = [];
  for (const relativeRoot of roots) {
    const absoluteRoot = path.join(ROOT, relativeRoot);
    if (!fs.existsSync(absoluteRoot)) continue;
    if (fs.statSync(absoluteRoot).isFile()) {
      files.push(absoluteRoot);
      continue;
    }
    const visit = (directory: string) => {
      for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        const absolute = path.join(directory, entry.name);
        if (entry.isDirectory()) visit(absolute);
        else if (entry.isFile()) files.push(absolute);
      }
    };
    visit(absoluteRoot);
  }
  return files;
}

function localTaskConfiguration(): string {
  const packageJson = JSON.parse(read("package.json")) as {
    scripts: Record<string, string>;
  };
  return [
    read("mise.toml"),
    JSON.stringify(packageJson.scripts),
    ...fs
      .readdirSync(path.join(ROOT, ".mise/tasks"))
      .filter((name) => name.endsWith(".toml"))
      .map((name) => read(path.posix.join(".mise/tasks", name))),
  ].join("\n");
}

describe("local build boundary", () => {
  it("removes every local cross-OS build entrypoint and dedicated contract", () => {
    for (const retiredPath of RETIRED_PATHS) {
      expect(fs.existsSync(path.join(ROOT, retiredPath))).toBe(false);
    }
    expect(
      fs.existsSync(path.join(ROOT, "tests/macosCrossWorkflow.test.ts")),
    ).toBe(false);
    expect(
      fs.existsSync(
        path.join(ROOT, ".trellis/spec/backend/wsl-macos-cross-build.md"),
      ),
    ).toBe(false);

    const taskSources = localTaskConfiguration();
    for (const task of RETIRED_TASKS) {
      expect(taskSources).not.toContain(`[tasks."${task}"]`);
      expect(taskSources).not.toContain(`["${task}"]`);
    }
    for (const retiredPath of RETIRED_PATHS) {
      expect(taskSources).not.toContain(retiredPath);
    }
    expect(taskSources).not.toContain("llvm-tools");
    expect(taskSources).not.toMatch(/^targets\s*=/m);

    const lock = read("mise.lock");
    expect(lock).not.toContain("llvm-tools");
    expect(lock).not.toMatch(/^targets\s*=/m);
  });

  it("keeps standard local development, builds, and tests current-host-only", () => {
    const packageJson = JSON.parse(read("package.json")) as {
      scripts: Record<string, string>;
    };
    expect(packageJson.scripts.dev).toBe(
      "node scripts/tasks/host-native.mjs dev",
    );
    expect(packageJson.scripts.build).toBe(
      "node scripts/tasks/host-native.mjs build",
    );
    expect(packageJson.scripts.tauri).toBe("tauri");

    const nativeTasks = read(".mise/tasks/core.toml");
    for (const task of ["dev", "build", "build:binary", "build:debug"]) {
      expect(nativeTasks).toContain(
        task.includes(":") ? `["${task}"]` : `[${task}]`,
      );
    }
    expect(nativeTasks).not.toContain("--target");
    expect(nativeTasks).toContain(
      'run = "node scripts/tasks/host-native.mjs build:binary"',
    );
    expect(nativeTasks).toContain(
      'run = "node scripts/tasks/host-native.mjs build:debug"',
    );
    const rustTasks = read(".mise/tasks/rust.toml");
    for (const operation of ["check", "clippy", "test"]) {
      expect(rustTasks).toContain(
        `run = "node scripts/tasks/rust.mjs ${operation}"`,
      );
    }

    const localEntryPoints = localTaskConfiguration();
    for (const marker of LOCAL_CROSS_EXECUTION_MARKERS) {
      expect(localEntryPoints, marker).not.toContain(marker);
    }

    for (const document of CURRENT_DOCUMENTS.slice(0, 4)) {
      const content = read(document);
      expect(content).toContain("mise run dev");
      expect(content).toContain("mise run build");
      expect(content).not.toMatch(/mise exec -- pnpm (?:dev|build)/);
      expect(content).not.toContain("dist-bundle/");
    }
  });

  it("maps only the six supported process hosts and verifies rustc identity", () => {
    const cases = [
      ["linux", "x64", "x86_64-unknown-linux-gnu"],
      ["linux", "arm64", "aarch64-unknown-linux-gnu"],
      ["darwin", "x64", "x86_64-apple-darwin"],
      ["darwin", "arm64", "aarch64-apple-darwin"],
      ["win32", "x64", "x86_64-pc-windows-msvc"],
      ["win32", "arm64", "aarch64-pc-windows-msvc"],
    ] as const;
    expect(Object.keys(hostNative.HOST_RUST_TARGETS)).toHaveLength(6);
    for (const [platform, architecture, target] of cases) {
      expect(hostNative.expectedRustTarget(platform, architecture)).toBe(
        target,
      );
      expect(
        hostNative.assertCurrentRustHost({
          platform,
          architecture,
          rustcVerboseVersion: `rustc 1.97.1\nhost: ${target}\nrelease: 1.97.1`,
        }),
      ).toBe(target);
    }
    expect(() => hostNative.expectedRustTarget("freebsd", "x64")).toThrow(
      "Unsupported local host OS/architecture",
    );
    expect(() =>
      hostNative.assertCurrentRustHost({
        platform: "linux",
        architecture: "x64",
        rustcVerboseVersion: "host: aarch64-unknown-linux-gnu",
      }),
    ).toThrow("does not match current host");
    expect(() => hostNative.parseRustcHost("rustc 1.97.1")).toThrow(
      "exactly one host target",
    );
  });

  it("rejects caller-owned target environment and Rust flag tokens", () => {
    const targetOverrideEnvironments: Array<Record<string, string>> = [
      { CARGO_BUILD_TARGET: "" },
      { TAURI_ENV_TARGET_TRIPLE: "x86_64-unknown-linux-gnu" },
      { Rustc: "/tmp/not-the-canonical-rustc" },
      { cargo_build_rustc: "/tmp/not-the-canonical-rustc" },
      { RUSTC_WRAPPER: "/tmp/not-a-wrapper" },
      { cargo_build_rustc_workspace_wrapper: "/tmp/not-a-wrapper" },
      { RUSTDOC: "/tmp/not-the-canonical-rustdoc" },
      { cargo_build_rustdoc: "/tmp/not-the-canonical-rustdoc" },
      { Cargo_Target_X86_64_Unknown_Linux_Gnu_Runner: "/tmp/emulator" },
      { cargo_target_x86_64_unknown_linux_gnu_linker: "/tmp/linker" },
      { LD_PRELOAD: "/tmp/inject.so" },
      { node_options: "--require=/tmp/inject.js" },
      { RUSTFLAGS: "-Dwarnings --target aarch64-unknown-linux-gnu" },
      {
        cargo_target_x86_64_unknown_linux_gnu_rustflags:
          "-Dwarnings --target aarch64-unknown-linux-gnu",
      },
      {
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS:
          "-C linker=/tmp/linker",
      },
      {
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTDOCFLAGS:
          "-C link-arg=/tmp/inject",
      },
      {
        CARGO_ENCODED_RUSTFLAGS:
          "-Dwarnings\u001f--target=aarch64-unknown-linux-gnu",
      },
      {
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTDOCFLAGS:
          "--target=aarch64-unknown-linux-gnu",
      },
    ];
    for (const environment of targetOverrideEnvironments) {
      expect(() =>
        hostNative.assertNoCallerTargetOverride(environment),
      ).toThrow(/canonical local tasks (?:own|reject)/);
    }
    expect(hostNative.containsRustTargetOverride("-Dwarnings --cfg test")).toBe(
      false,
    );
    expect(() =>
      hostNative.assertNoCallerTargetOverride({
        RUSTFLAGS: "-Dwarnings --cfg test",
      }),
    ).not.toThrow();
  });

  it("plans fixed Tauri and Cargo argv with the verified current host target", () => {
    const base = {
      environment: {},
      platform: "linux",
      architecture: "x64",
      rustcVerboseVersion:
        "rustc 1.97.1\ncommit-hash: verified-toolchain\nhost: x86_64-unknown-linux-gnu\nrelease: 1.97.1",
      rustdocVerboseVersion:
        "rustdoc 1.97.1\ncommit-hash: verified-toolchain\nhost: x86_64-unknown-linux-gnu\nrelease: 1.97.1",
      rustcExecutable: "/toolchain/bin/rustc",
      rustdocExecutable: "/toolchain/bin/rustdoc",
      nativeRunnerConfig:
        'target.x86_64-unknown-linux-gnu.runner=["/usr/bin/node","/repo/scripts/tasks/host-native.mjs","native-runner","x86_64-unknown-linux-gnu"]',
    };
    const dev = hostNative.planTauriTask({ ...base, operation: "dev" });
    expect(dev).toMatchObject({
      command: "pnpm",
      args: ["tauri", "dev", "--target", "x86_64-unknown-linux-gnu"],
      target: "x86_64-unknown-linux-gnu",
    });
    expect(dev.environment).toMatchObject({
      RUSTC: "/toolchain/bin/rustc",
      CARGO_BUILD_RUSTC: "/toolchain/bin/rustc",
      RUSTDOC: "/toolchain/bin/rustdoc",
      CARGO_BUILD_RUSTDOC: "/toolchain/bin/rustdoc",
      RUSTC_WRAPPER: "",
      RUSTC_WORKSPACE_WRAPPER: "",
      RUSTFLAGS: "",
      CARGO_ENCODED_RUSTFLAGS: "",
      RUSTDOCFLAGS: "",
      CARGO_ENCODED_RUSTDOCFLAGS: "",
    });
    expect(
      hostNative.planTauriTask({ ...base, operation: "build:binary" }).args,
    ).toEqual([
      "tauri",
      "build",
      "--target",
      "x86_64-unknown-linux-gnu",
      "--no-bundle",
    ]);
    expect(
      hostNative.planTauriTask({ ...base, operation: "build:debug" }).args,
    ).toEqual([
      "tauri",
      "build",
      "--target",
      "x86_64-unknown-linux-gnu",
      "--debug",
    ]);
    expect(
      hostNative.planCargoTask({ ...base, operation: "check" }).args,
    ).toEqual([
      "--config",
      base.nativeRunnerConfig,
      "check",
      "--target",
      "x86_64-unknown-linux-gnu",
      "--workspace",
      "--locked",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--all-targets",
    ]);
    expect(
      hostNative.planCargoTask({
        ...base,
        operation: "test",
        filters: ["settings"],
      }).args,
    ).toEqual([
      "--config",
      base.nativeRunnerConfig,
      "test",
      "--target",
      "x86_64-unknown-linux-gnu",
      "--workspace",
      "--locked",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--",
      "settings",
    ]);
    expect(
      hostNative
        .planCargoTask({
          ...base,
          operation: "test",
          filters: ["settings&whoami|ignored"],
        })
        .args.slice(-2),
    ).toEqual(["--", "settings&whoami|ignored"]);
    expect(() =>
      hostNative.planTauriTask({
        ...base,
        operation: "build",
        forwardedArguments: ["--target", "aarch64-unknown-linux-gnu"],
      }),
    ).toThrow("does not accept forwarded arguments");
  });

  it("encodes the no-shell native runner as fixed argv even when Windows paths contain spaces", () => {
    const nodeExecutable = "C:\\Program Files\\Node JS\\node.exe";
    const runnerScript =
      "C:\\Work Space\\FyAgent\\scripts\\tasks\\host-native.mjs";
    const runner = hostNative.buildNativeRunnerConfig({
      target: "x86_64-pc-windows-msvc",
      platform: "win32",
      nodeExecutable,
      runnerScript,
    });
    expect(runner).toBe(
      `target.x86_64-pc-windows-msvc.runner=${JSON.stringify([
        nodeExecutable,
        runnerScript,
        "native-runner",
        "x86_64-pc-windows-msvc",
      ])}`,
    );
    expect(runner).not.toMatch(
      /cmd(?:\.exe)?|powershell|pwsh|(?:^|\s)-c(?:\s|$)/i,
    );
  });

  it("prevents repository tasks, scripts, and hooks from changing mise trust", () => {
    const miseTrustMutation =
      /(?:\bmise(?:\.exe)?\b|\/[^\s"'`]*\/mise\b|[A-Za-z]:\\[^\s"'`]*\\mise(?:\.exe)?\b|\$\{?[A-Za-z_][A-Za-z0-9_]*MISE[A-Za-z0-9_]*\}?)[^\r\n]*\b(?:trust|untrust)\b/i;
    for (const file of executableRepositoryFiles()) {
      const relative = path.relative(ROOT, file);
      const content = read(relative);
      expect(content, relative).not.toMatch(miseTrustMutation);
    }
  });

  it("keeps current documents free of retired cross-build interfaces", () => {
    for (const document of CURRENT_DOCUMENTS) {
      const content = read(document);
      for (const task of RETIRED_TASKS) {
        expect(content, document).not.toContain(task);
      }
      for (const retiredPath of RETIRED_PATHS) {
        expect(content, document).not.toContain(retiredPath);
      }
      expect(content, document).not.toContain("wsl-macos-cross-build.md");
    }
  });

  it("retains native release targets for all five platform groups", () => {
    const release = read(".github/workflows/release.yml");
    for (const contract of [
      "runner: windows-2022",
      "target_group: windows-x64",
      "runner: windows-11-arm",
      "target_group: windows-arm64",
      "runner: ubuntu-24.04",
      "target_group: linux-x64",
      "runner: ubuntu-24.04-arm",
      "target_group: linux-arm64",
      "runs-on: macos-15",
      "TARGET_GROUP: macos-universal",
    ]) {
      expect(release).toContain(contract);
    }

    expect(release).toContain(
      "image: ${{ matrix.container_image }}@${{ matrix.container_digest }}",
    );
    expect(
      release.match(/container_image: docker\.io\/library\/ubuntu:22\.04/g),
    ).toHaveLength(2);
    expect(release).toContain(
      "sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e",
    );
    expect(release).toContain(
      "sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149",
    );
    expect(release).toContain("aarch64-pc-windows-msvc");
    expect(release).toContain("pnpm tauri build --no-bundle");
    expect(release).toContain("pnpm tauri build --bundles appimage,deb,rpm");
    expect(release).toContain(
      "pnpm tauri build --target universal-apple-darwin",
    );
    expect(release).toContain("FYAGENT_WINDOWS_MANIFEST: release");
  });
});
