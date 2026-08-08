import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as taskLibModule from "../scripts/tasks/lib.mjs";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as hostNativeModule from "../scripts/tasks/host-native.mjs";

const ROOT = path.resolve(__dirname, "..");

type TaskDefinition = {
  confirm?: { default?: string; message?: string };
  env: { FYAGENT_TASK_EFFECT: string };
  interactive?: boolean;
  raw?: boolean;
  usage?: string;
};

type ContractModule = {
  PARAMETERIZED_TASKS: readonly string[];
  RAW_TASKS: readonly string[];
  loadTaskDefinitions(): Record<string, TaskDefinition>;
};

type LockAsset = {
  checksum: string;
  url: string;
};

const readToml = taskLibModule.readToml as (relativePath: string) => unknown;
const resolveTaskExecutable = taskLibModule.resolveTaskExecutable as (
  command: string,
  platform?: NodeJS.Platform,
) => string;

function mise(...args: string[]) {
  return spawnSync("mise", ["run", ...args], {
    cwd: ROOT,
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1" },
  });
}

function taskEnvironment(overrides: Record<string, string>) {
  const environment = { ...process.env };
  const controlled = new Set(
    [
      "CARGO_BUILD_TARGET",
      "TAURI_ENV_TARGET_TRIPLE",
      "RUSTC",
      "CARGO_BUILD_RUSTC",
      "RUSTC_WRAPPER",
      "CARGO_BUILD_RUSTC_WRAPPER",
      "RUSTC_WORKSPACE_WRAPPER",
      "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
      "RUSTDOC",
      "CARGO_BUILD_RUSTDOC",
      "RUSTFLAGS",
      "CARGO_BUILD_RUSTFLAGS",
      "CARGO_ENCODED_RUSTFLAGS",
      "RUSTDOCFLAGS",
      "CARGO_BUILD_RUSTDOCFLAGS",
      "CARGO_ENCODED_RUSTDOCFLAGS",
      "LD_PRELOAD",
      "LD_LIBRARY_PATH",
      "DYLD_INSERT_LIBRARIES",
      "DYLD_FORCE_FLAT_NAMESPACE",
      "DYLD_LIBRARY_PATH",
      "DYLD_FRAMEWORK_PATH",
      "DYLD_FALLBACK_LIBRARY_PATH",
      "NODE_OPTIONS",
      "NODE_PATH",
    ].map((name) => name.toUpperCase()),
  );
  for (const name of Object.keys(environment)) {
    const normalized = name.toUpperCase();
    if (
      controlled.has(normalized) ||
      /^CARGO_TARGET_.+_(?:RUNNER|LINKER|RUSTFLAGS|RUSTDOCFLAGS)$/.test(
        normalized,
      )
    ) {
      delete environment[name];
    }
  }
  return { ...environment, NO_COLOR: "1", ...overrides };
}

function foreignRustTarget(): string {
  const current = hostNativeModule.expectedRustTarget(
    process.platform,
    process.arch,
  ) as string;
  const foreign = Object.values(
    hostNativeModule.HOST_RUST_TARGETS as Record<string, string>,
  ).find((target) => target !== current);
  if (!foreign) throw new Error("No foreign Rust target fixture is available");
  return foreign;
}

function output(result: ReturnType<typeof mise>): string {
  return `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
}

function digest(relativePath: string): string {
  return createHash("sha256")
    .update(fs.readFileSync(path.join(ROOT, relativePath)))
    .digest("hex");
}

describe("canonical mise task API", () => {
  it("uses mise's native Windows pnpm executable without changing other commands", () => {
    expect(resolveTaskExecutable("pnpm", "win32")).toBe("pnpm.exe");
    expect(resolveTaskExecutable("pnpm", "linux")).toBe("pnpm");
    expect(resolveTaskExecutable("pnpm", "darwin")).toBe("pnpm");

    for (const command of ["npm", "npx", "pnpx", "node", "cargo"]) {
      expect(resolveTaskExecutable(command, "win32"), command).toBe(command);
    }
  });

  it("locks native pnpm executables and checksums for both Windows architectures", () => {
    const lock = readToml("mise.lock") as {
      tools: {
        pnpm: Array<Record<string, LockAsset>>;
      };
    };
    expect(lock.tools.pnpm).toHaveLength(1);
    const [pnpm] = lock.tools.pnpm;
    for (const [platform, assetName] of [
      ["windows-x64", "pnpm-win-x64.exe"],
      ["windows-arm64", "pnpm-win-arm64.exe"],
    ] as const) {
      const asset = pnpm[`platforms.${platform}`];
      expect(asset.url.endsWith(`/${assetName}`), platform).toBe(true);
      expect(asset.checksum, platform).toMatch(/^sha256:[0-9a-f]{64}$/);
    }
  });

  it("loads a complete and extensible catalog with valid metadata", () => {
    const validation = spawnSync(
      "mise",
      ["tasks", "validate", "--errors-only"],
      {
        cwd: ROOT,
        encoding: "utf8",
      },
    );
    expect(validation.status).toBe(0);
    expect(output(validation)).toContain("task(s) validated successfully");

    const contract = spawnSync(
      process.execPath,
      ["scripts/tasks/task-contract-check.mjs"],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(contract.status, output(contract)).toBe(0);
    const report = JSON.parse(contract.stdout) as {
      ok: boolean;
      tasks: number;
      checkClosure: string[];
    };
    expect(report.ok).toBe(true);
    expect(report.tasks).toBeGreaterThanOrEqual(80);
    expect(report.checkClosure).toContain("check:contracts");
  });

  it("enforces usage, mutation, interactive, raw, and confirmation metadata", async () => {
    const contract = (await import(
      /* @vite-ignore */ pathToFileURL(
        path.join(ROOT, "scripts", "tasks", "task-contract-check.mjs"),
      ).href
    )) as ContractModule;
    const tasks = contract.loadTaskDefinitions();

    for (const name of contract.PARAMETERIZED_TASKS) {
      expect(tasks[name].usage?.trim(), name).toBeTruthy();
    }
    for (const [name, task] of Object.entries(tasks)) {
      const effect = task.env.FYAGENT_TASK_EFFECT;
      if (effect === "preview-by-default") {
        expect(task.usage, name).toContain('flag "--apply"');
      }
      expect(task.interactive === true, name).toBe(effect === "interactive");
    }
    expect(
      Object.entries(tasks)
        .filter(([, task]) => task.raw === true)
        .map(([name]) => name)
        .sort(),
    ).toEqual([...contract.RAW_TASKS].sort());
    expect(tasks["upstream:merge:abort"]).toMatchObject({
      confirm: { default: "no" },
      env: { FYAGENT_TASK_EFFECT: "git-state" },
    });
  });

  it("forwards a unit-test file filter through the real mise usage parser", () => {
    const result = mise("test:unit", "tests/developmentEnvironment.test.ts");
    expect(result.status, output(result)).toBe(0);
    expect(output(result)).toContain("developmentEnvironment.test.ts");
    expect(output(result)).not.toContain("miseTaskContract.test.ts");
  }, 60_000);

  it("forwards version and Python parameters while preview mode preserves files", () => {
    const guardedFiles = [
      "src-tauri/Cargo.toml",
      "src-tauri/Cargo.lock",
      "pyproject.toml",
      "uv.lock",
    ];
    const before = new Map(
      guardedFiles.map((relativePath) => [relativePath, digest(relativePath)]),
    );

    const version = mise("version:set", "0.3.0");
    expect(version.status, output(version)).toBe(0);
    expect(output(version)).toContain("0.3.0");
    expect(output(version)).toMatch(/preview|no files (?:would )?change/i);

    const python = mise("python:add:dev", "httpx");
    expect(python.status, output(python)).toBe(0);
    expect(output(python)).toContain("httpx");
    expect(output(python)).toContain('"status": "preview"');

    for (const relativePath of guardedFiles) {
      expect(digest(relativePath), relativePath).toBe(before.get(relativePath));
    }
  });

  it("forwards upstream parameters before any Git mutation can run", () => {
    const result = mise("upstream:merge:prepare", "not-a-release-tag");
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("Upstream tag must be exact vX.Y.Z");
    expect(output(result)).not.toContain("git merge");
  });

  it("forwards flags to the JSON environment report", () => {
    const result = mise("env:check", "--json");
    expect(result.status, output(result)).toBe(0);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      checks: Array<{ name: string; ok: boolean }>;
    };
    expect(report.ok).toBe(true);
    expect(report.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "Node ownership", ok: true }),
        expect.objectContaining({
          name: "Rust toolchain and components",
          ok: true,
        }),
      ]),
    );
  });

  it("validates every active Trellis task when no task is specified", () => {
    const result = mise("trellis:validate");
    expect(result.status, output(result)).toBe(0);
    expect(output(result)).toMatch(/Validated \d+ active Trellis task\(s\)/);
    expect(output(result)).not.toContain(
      "the following arguments are required",
    );
  }, 60_000);

  it("forwards the task CLI help flag through the canonical wrapper", () => {
    const result = mise("trellis:task", "--", "--help");
    expect(result.status, output(result)).toBe(0);
    expect(output(result)).toContain("usage: task.py");
    expect(output(result)).toContain("{create,add-context,validate");
  });

  it.each([
    ["split target", ["--", "--target", "aarch64-unknown-linux-gnu"]],
    ["equals target", ["--", "--target=aarch64-unknown-linux-gnu"]],
  ])("rejects %s injection before Cargo runs", (_label, args) => {
    const result = mise("rust:test", ...args);
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("Cargo options and targets are forbidden");
    expect(output(result)).not.toMatch(/Compiling|Finished.*test profile/);
  });

  it("plans the real wrapper runtime with rustc identity before launching the fixed host command", () => {
    const target = hostNativeModule.expectedRustTarget(
      process.platform,
      process.arch,
    ) as string;
    const rustcExecutable = "/verified/toolchain/bin/rustc";
    const rustdocExecutable = "/verified/toolchain/bin/rustdoc";
    const verbose = (tool: "rustc" | "rustdoc") =>
      `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1`;
    const calls: Array<{
      command: string;
      args: string[];
      environment?: Record<string, string>;
    }> = [];
    const captureCommand = (command: string, args: string[]) => {
      calls.push({ command, args });
      return command === rustcExecutable
        ? verbose("rustc")
        : verbose("rustdoc");
    };
    const runCommand = (
      command: string,
      args: string[],
      options: { env: Record<string, string> },
    ) => {
      calls.push({ command, args, environment: options.env });
    };
    const resolveToolCommand = ({ tool }: { tool: string }) =>
      tool === "rustc" ? rustcExecutable : rustdocExecutable;
    const nativeRunnerConfig = `target.${target}.runner=["/verified/node","/verified/host-native.mjs","native-runner","${target}"]`;
    const resolveRunner = () => nativeRunnerConfig;

    const tauri = hostNativeModule.executeTauriTask({
      operation: "build:debug",
      environment: {},
      platform: process.platform,
      architecture: process.arch,
      captureCommand,
      runCommand,
      resolveToolCommand,
    }) as { command: string; args: string[]; target: string };
    expect(tauri).toMatchObject({
      command: "pnpm",
      args: ["tauri", "build", "--target", target, "--debug"],
      target,
    });
    expect(calls.map(({ command, args }) => ({ command, args }))).toEqual([
      { command: rustcExecutable, args: ["-vV"] },
      { command: rustdocExecutable, args: ["-vV"] },
      { command: "pnpm", args: tauri.args },
    ]);
    expect(calls[2].environment).toMatchObject({
      RUSTC: rustcExecutable,
      CARGO_BUILD_RUSTC: rustcExecutable,
      RUSTDOC: rustdocExecutable,
      CARGO_BUILD_RUSTDOC: rustdocExecutable,
      RUSTC_WRAPPER: "",
      RUSTC_WORKSPACE_WRAPPER: "",
      RUSTFLAGS: "",
      CARGO_ENCODED_RUSTFLAGS: "",
      RUSTDOCFLAGS: "",
      CARGO_ENCODED_RUSTDOCFLAGS: "",
    });

    calls.length = 0;
    const cargo = hostNativeModule.executeCargoTask({
      operation: "test",
      filters: ["settings"],
      environment: {},
      platform: process.platform,
      architecture: process.arch,
      captureCommand,
      runCommand,
      resolveToolCommand,
      resolveRunner,
    }) as { command: string; args: string[]; target: string };
    expect(cargo.command).toBe("cargo");
    expect(cargo.args).toEqual([
      "--config",
      nativeRunnerConfig,
      "test",
      "--target",
      target,
      "--workspace",
      "--locked",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--",
      "settings",
    ]);
    expect(calls.map(({ command, args }) => ({ command, args }))).toEqual([
      { command: rustcExecutable, args: ["-vV"] },
      { command: rustdocExecutable, args: ["-vV"] },
      { command: "cargo", args: cargo.args },
    ]);
    expect(calls[2].environment).toMatchObject({
      RUSTC: rustcExecutable,
      RUSTDOC: rustdocExecutable,
    });
  });

  it.runIf(process.platform !== "win32")(
    "rejects effective Cargo toolchain config includes, cycles, and symlinks before tools start",
    () => {
      const fixture = fs.mkdtempSync(
        path.join(os.tmpdir(), "fyagent-cargo-config-contract-"),
      );
      const configDirectory = path.join(fixture, ".cargo");
      const configFile = path.join(configDirectory, "config.toml");
      const includedFile = path.join(configDirectory, "included.toml");
      const cargoHome = path.join(fixture, "cargo-home");
      fs.mkdirSync(configDirectory, { recursive: true });
      fs.mkdirSync(cargoHome, { recursive: true });

      try {
        fs.writeFileSync(
          configFile,
          'include = [{ path = "included.toml", optional = true }]\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).not.toThrow();

        fs.writeFileSync(
          includedFile,
          '[target.x86_64-unknown-linux-gnu]\nrunner = "/tmp/emulator"\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/target\.x86_64-unknown-linux-gnu\.runner is forbidden/);

        fs.writeFileSync(
          includedFile,
          '[target.x86_64-unknown-linux-gnu]\nlinker = "/tmp/linker"\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/target\.x86_64-unknown-linux-gnu\.linker is forbidden/);

        fs.writeFileSync(
          includedFile,
          '[target.x86_64-unknown-linux-gnu]\nrustflags = ["-C", "linker=/tmp/linker"]\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/target\.x86_64-unknown-linux-gnu\.rustflags is forbidden/);

        fs.writeFileSync(
          includedFile,
          '[env]\nnode_options = { value = "--require=/tmp/inject.js", force = true }\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/env\.node_options is forbidden/);

        fs.writeFileSync(includedFile, 'include = ["config.toml"]\n');
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/include cycle is forbidden/);

        fs.writeFileSync(includedFile, "[build]\nrustflags = []\n");
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/build\.rustflags is forbidden/);

        fs.rmSync(configFile);
        fs.symlinkSync(includedFile, configFile);
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/regular non-symlink file/);
      } finally {
        fs.rmSync(fixture, { recursive: true, force: true });
      }
    },
  );

  it.runIf(process.platform === "linux")(
    "passes verified host argv through real wrappers to fake native executables",
    () => {
      const fakeBin = fs.mkdtempSync(
        path.join(os.tmpdir(), "fyagent-host-native-smoke-"),
      );
      const marker = path.join(fakeBin, "calls.jsonl");
      const target = hostNativeModule.expectedRustTarget(
        process.platform,
        process.arch,
      ) as string;
      const nativeRunnerConfig = hostNativeModule.buildNativeRunnerConfig({
        target,
        platform: process.platform,
        nodeExecutable: process.execPath,
        runnerScript: path.join(ROOT, "scripts", "tasks", "host-native.mjs"),
      }) as string;
      const ownedEnvironment = [
        "RUSTC",
        "CARGO_BUILD_RUSTC",
        "RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "RUSTDOC",
        "CARGO_BUILD_RUSTDOC",
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTDOCFLAGS",
        "CARGO_BUILD_RUSTDOCFLAGS",
        "CARGO_ENCODED_RUSTDOCFLAGS",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_FORCE_FLAT_NAMESPACE",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "NODE_OPTIONS",
        "NODE_PATH",
      ];
      const executable = (name: string, body: string) => {
        const file = path.join(fakeBin, name);
        fs.writeFileSync(file, `#!${process.execPath}\n${body}`, {
          mode: 0o755,
        });
      };
      const record = (command: string) =>
        `require("node:fs").appendFileSync(${JSON.stringify(marker)}, JSON.stringify({ command: ${JSON.stringify(command)}, args: process.argv.slice(2), environment: Object.fromEntries(${JSON.stringify(ownedEnvironment)}.map((name) => [name, process.env[name]])) }) + "\\n");\n`;
      executable(
        "rustc",
        `${record("rustc")}process.stdout.write(${JSON.stringify(
          `rustc 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1\n`,
        )});\n`,
      );
      executable(
        "rustdoc",
        `${record("rustdoc")}process.stdout.write(${JSON.stringify(
          `rustdoc 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1\n`,
        )});\n`,
      );
      executable("pnpm", record("pnpm"));
      executable("cargo", record("cargo"));
      const targetDirectory = path.join(ROOT, "src-tauri", "target", target);
      const targetDirectoryExisted = fs.existsSync(targetDirectory);
      let nativeFixtureDirectory: string | undefined;

      try {
        const environment = taskEnvironment({ PATH: fakeBin });
        const tauri = spawnSync(
          process.execPath,
          ["scripts/tasks/host-native.mjs", "build:binary"],
          { cwd: ROOT, encoding: "utf8", env: environment },
        );
        expect(tauri.status, output(tauri)).toBe(0);
        const cargo = spawnSync(
          process.execPath,
          ["scripts/tasks/rust.mjs", "clippy"],
          { cwd: ROOT, encoding: "utf8", env: environment },
        );
        expect(cargo.status, output(cargo)).toBe(0);

        const calls = fs
          .readFileSync(marker, "utf8")
          .trim()
          .split("\n")
          .map(
            (line) =>
              JSON.parse(line) as {
                command: string;
                args: string[];
                environment: Record<string, string>;
              },
          );
        expect(calls.map(({ command, args }) => ({ command, args }))).toEqual([
          { command: "rustc", args: ["-vV"] },
          { command: "rustdoc", args: ["-vV"] },
          {
            command: "pnpm",
            args: ["tauri", "build", "--target", target, "--no-bundle"],
          },
          { command: "rustc", args: ["-vV"] },
          { command: "rustdoc", args: ["-vV"] },
          {
            command: "cargo",
            args: [
              "--config",
              nativeRunnerConfig,
              "clippy",
              "--target",
              target,
              "--workspace",
              "--locked",
              "--manifest-path",
              "src-tauri/Cargo.toml",
              "--all-targets",
              "--",
              "-D",
              "warnings",
            ],
          },
        ]);
        for (const index of [2, 5]) {
          expect(calls[index].environment).toMatchObject({
            RUSTC: path.join(fakeBin, "rustc"),
            CARGO_BUILD_RUSTC: path.join(fakeBin, "rustc"),
            RUSTDOC: path.join(fakeBin, "rustdoc"),
            CARGO_BUILD_RUSTDOC: path.join(fakeBin, "rustdoc"),
            RUSTC_WRAPPER: "",
            CARGO_BUILD_RUSTC_WRAPPER: "",
            RUSTC_WORKSPACE_WRAPPER: "",
            CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER: "",
            RUSTFLAGS: "",
            CARGO_BUILD_RUSTFLAGS: "",
            CARGO_ENCODED_RUSTFLAGS: "",
            RUSTDOCFLAGS: "",
            CARGO_BUILD_RUSTDOCFLAGS: "",
            CARGO_ENCODED_RUSTDOCFLAGS: "",
            LD_PRELOAD: "",
            NODE_OPTIONS: "",
          });
        }

        fs.mkdirSync(targetDirectory, { recursive: true });
        nativeFixtureDirectory = fs.mkdtempSync(
          path.join(targetDirectory, "fyagent-native-runner-smoke-"),
        );
        const nativeFixture = path.join(nativeFixtureDirectory, "printf");
        fs.copyFileSync("/usr/bin/printf", nativeFixture);
        fs.chmodSync(nativeFixture, 0o755);
        const metacharacterFilter = "filter&whoami|ignored";
        const direct = spawnSync(
          process.execPath,
          [
            "scripts/tasks/host-native.mjs",
            "native-runner",
            target,
            nativeFixture,
            "%s",
            metacharacterFilter,
          ],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(direct.status, output(direct)).toBe(0);
        expect(direct.stdout).toBe(metacharacterFilter);

        const outside = spawnSync(
          process.execPath,
          [
            "scripts/tasks/host-native.mjs",
            "native-runner",
            target,
            "/usr/bin/printf",
          ],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(outside.status).not.toBe(0);
        expect(output(outside)).toContain(
          "verified current-host target directory",
        );

        const symlink = path.join(nativeFixtureDirectory, "symlink");
        fs.symlinkSync("/usr/bin/printf", symlink);
        const linked = spawnSync(
          process.execPath,
          ["scripts/tasks/host-native.mjs", "native-runner", target, symlink],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(linked.status).not.toBe(0);
        expect(output(linked)).toContain("regular non-symlink file");

        const invalid = path.join(nativeFixtureDirectory, "not-elf");
        fs.writeFileSync(invalid, "not a native executable", { mode: 0o755 });
        const wrongSignature = spawnSync(
          process.execPath,
          ["scripts/tasks/host-native.mjs", "native-runner", target, invalid],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(wrongSignature.status).not.toBe(0);
        expect(output(wrongSignature)).toContain("ELF header");

        const wrongArchitecture = path.join(
          nativeFixtureDirectory,
          "wrong-architecture",
        );
        const wrongArchitectureBytes = fs.readFileSync(nativeFixture);
        wrongArchitectureBytes.writeUInt16LE(
          process.arch === "x64" ? 183 : 62,
          18,
        );
        fs.writeFileSync(wrongArchitecture, wrongArchitectureBytes, {
          mode: 0o755,
        });
        const wrongMachine = spawnSync(
          process.execPath,
          [
            "scripts/tasks/host-native.mjs",
            "native-runner",
            target,
            wrongArchitecture,
          ],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(wrongMachine.status).not.toBe(0);
        expect(output(wrongMachine)).toContain("does not match");
      } finally {
        fs.rmSync(fakeBin, { recursive: true, force: true });
        if (nativeFixtureDirectory) {
          fs.rmSync(nativeFixtureDirectory, { recursive: true, force: true });
        }
        if (!targetDirectoryExisted && fs.existsSync(targetDirectory)) {
          fs.rmdirSync(targetDirectory);
        }
      }
    },
  );

  it("rejects caller target controls before rustc, Cargo, or Tauri can start", () => {
    let childCalls = 0;
    const forbiddenChild = () => {
      childCalls += 1;
      throw new Error("child command must not start");
    };
    for (const environment of [
      { CARGO_BUILD_TARGET: foreignRustTarget() },
      { TAURI_ENV_TARGET_TRIPLE: foreignRustTarget() },
      { Rustc: "/tmp/not-the-canonical-rustc" },
      { cargo_build_rustc: "/tmp/not-the-canonical-rustc" },
      { RUSTC_WRAPPER: "/tmp/not-a-wrapper" },
      { cargo_build_rustc_workspace_wrapper: "/tmp/not-a-wrapper" },
      { RUSTDOC: "/tmp/not-the-canonical-rustdoc" },
      { cargo_build_rustdoc: "/tmp/not-the-canonical-rustdoc" },
      { Cargo_Target_X86_64_Unknown_Linux_Gnu_Runner: "/tmp/emulator" },
      { cargo_target_x86_64_unknown_linux_gnu_linker: "/tmp/linker" },
      { LD_PRELOAD: "/tmp/inject.so" },
      { dyld_library_path: "/tmp/inject" },
      { NODE_OPTIONS: "--require=/tmp/inject.js" },
      { RUSTFLAGS: `--target ${foreignRustTarget()}` },
      { CARGO_BUILD_RUSTFLAGS: `--target=${foreignRustTarget()}` },
      {
        cargo_target_x86_64_unknown_linux_gnu_rustflags: `-Dwarnings --target ${foreignRustTarget()}`,
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
        CARGO_ENCODED_RUSTFLAGS: `--target\u001f${foreignRustTarget()}`,
      },
      { RUSTDOCFLAGS: `--target=${foreignRustTarget()}` },
      {
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTDOCFLAGS: `--target ${foreignRustTarget()}`,
      },
    ]) {
      expect(() =>
        hostNativeModule.assertHostNativeGuard({
          environment,
          platform: process.platform,
          architecture: process.arch,
        }),
      ).toThrow(/canonical local tasks (?:own|reject)/);
      for (const operation of ["dev", "build", "build:binary", "build:debug"]) {
        expect(() =>
          hostNativeModule.executeTauriTask({
            operation,
            environment,
            captureCommand: forbiddenChild,
            runCommand: forbiddenChild,
          }),
        ).toThrow(/canonical local tasks (?:own|reject)/);
      }
      for (const operation of ["check", "clippy", "test"]) {
        expect(() =>
          hostNativeModule.executeCargoTask({
            operation,
            environment,
            captureCommand: forbiddenChild,
            runCommand: forbiddenChild,
          }),
        ).toThrow(/canonical local tasks (?:own|reject)/);
      }
    }
    for (const operation of ["dev", "build", "build:binary", "build:debug"]) {
      expect(() =>
        hostNativeModule.executeTauriTask({
          operation,
          forwardedArguments: ["--target", foreignRustTarget()],
          environment: {},
          captureCommand: forbiddenChild,
          runCommand: forbiddenChild,
        }),
      ).toThrow("does not accept forwarded arguments");
    }
    for (const operation of ["check", "clippy", "test"]) {
      expect(() =>
        hostNativeModule.executeCargoTask({
          operation,
          forwardedArguments: ["--target", foreignRustTarget()],
          environment: {},
          captureCommand: forbiddenChild,
          runCommand: forbiddenChild,
        }),
      ).toThrow("does not accept forwarded arguments");
    }
    expect(childCalls).toBe(0);
  });

  it.each([
    "dev",
    "build",
    "build:binary",
    "build:debug",
    "check",
    "rust:check",
    "rust:clippy",
    "rust:test",
  ])("fails mise run %s closed on a caller target environment", (task) => {
    const result = spawnSync("mise", ["run", task], {
      cwd: ROOT,
      encoding: "utf8",
      env: taskEnvironment({ CARGO_BUILD_TARGET: foreignRustTarget() }),
    });
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("CARGO_BUILD_TARGET must not be set");
    expect(output(result)).not.toMatch(
      /Compiling|Finished.*profile|beforeDevCommand|beforeBuildCommand/,
    );
  });

  it("fails aggregate check closed on compiler and runner overrides before env:check", () => {
    const overrideCases: Array<Record<string, string>> = [
      { Rustc: "/tmp/not-the-canonical-rustc" },
      { Cargo_Target_X86_64_Unknown_Linux_Gnu_Runner: "/tmp/emulator" },
    ];
    for (const overrides of overrideCases) {
      const result = spawnSync("mise", ["run", "check"], {
        cwd: ROOT,
        encoding: "utf8",
        env: taskEnvironment(overrides),
      });
      expect(result.status).not.toBe(0);
      expect(output(result)).toContain("canonical local tasks own");
      expect(output(result)).not.toContain("[env:check]");
    }
  });

  it("fails pnpm dev/build closed on forwarded args and target environment", () => {
    const target = foreignRustTarget();
    for (const [script, frontendCommand] of [
      ["dev", "beforeDevCommand"],
      ["build", "beforeBuildCommand"],
    ] as const) {
      const forwarded = spawnSync(
        resolveTaskExecutable("pnpm"),
        ["run", script, "--", "--target", target],
        { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
      );
      expect(forwarded.status).not.toBe(0);
      expect(output(forwarded)).toContain(
        "does not accept forwarded arguments",
      );
      expect(output(forwarded)).not.toContain(frontendCommand);
    }

    const build = spawnSync(resolveTaskExecutable("pnpm"), ["run", "build"], {
      cwd: ROOT,
      encoding: "utf8",
      env: taskEnvironment({ TAURI_ENV_TARGET_TRIPLE: target }),
    });
    expect(build.status).not.toBe(0);
    expect(output(build)).toContain("TAURI_ENV_TARGET_TRIPLE must not be set");
    expect(output(build)).not.toContain("beforeBuildCommand");
  });

  it.each(["--update", "--outputFile=vitest-results.json"])(
    "rejects the write-capable Vitest option %s before Vitest runs",
    (option) => {
      const result = mise("test:unit", "--", option);
      expect(result.status).not.toBe(0);
      expect(output(result)).toContain("Vitest options are forbidden");
      expect(output(result)).not.toContain("RUN ");
    },
  );
});
