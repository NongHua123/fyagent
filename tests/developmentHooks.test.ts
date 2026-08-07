import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(__dirname, "..");
const runnerPath = path.join(
  repositoryRoot,
  "scripts",
  "tasks",
  "codex-hook-runner.mjs",
);

type HookInput = Record<string, unknown>;
type HookOutput = Record<string, unknown>;
type SpawnResult = {
  error?: Error;
  signal: NodeJS.Signals | null;
  status: number;
  stderr: string;
  stdout: string;
};
type SpawnStub = (
  command: string,
  args: string[],
  options: Record<string, unknown>,
) => SpawnResult;
type RunnerModule = {
  executeHook(options: {
    projectRoot: string;
    mode: "workflow-state" | "subagent-context";
    input: HookInput;
    spawn?: SpawnStub;
    environment?: NodeJS.ProcessEnv;
  }): HookOutput;
  explicitDisable(environment: NodeJS.ProcessEnv): boolean;
  isPathInsideProject(
    projectRoot: string,
    candidate: string,
    pathImplementation?: typeof path.win32,
  ): boolean;
  parseInput(
    rawInput: string,
    mode: "workflow-state" | "subagent-context",
  ): HookInput;
  snapshotTree(projectRoot: string): Record<string, unknown>;
  validateProject(
    projectRoot: string,
    mode?: "workflow-state" | "subagent-context",
  ): { ready: boolean; reason?: string };
};

let runner: RunnerModule;
const fixtureRoots: string[] = [];

function writeFixture(root: string, relativePath: string, content: string) {
  const filePath = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

function createHookFixture({ ready = false }: { ready?: boolean } = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-hooks-test-"));
  fixtureRoots.push(root);
  fs.mkdirSync(path.join(root, ".trellis"), { recursive: true });
  writeFixture(root, ".python-version", "3.14.7\n");
  writeFixture(
    root,
    "pyproject.toml",
    [
      "[project]",
      'name = "fyagent-development-environment"',
      'version = "0.0.0"',
      'requires-python = ">=3.14,<3.15"',
      "dependencies = []",
      "",
      "[dependency-groups]",
      "dev = []",
      "",
      "[tool.uv]",
      "package = false",
      'python-preference = "only-managed"',
      'python-downloads = "automatic"',
      "",
    ].join("\n"),
  );
  writeFixture(
    root,
    "uv.lock",
    ["version = 1", "revision = 3", 'requires-python = "==3.14.*"', ""].join(
      "\n",
    ),
  );
  for (const hook of [
    "inject-workflow-state.py",
    "inject-subagent-context.py",
  ]) {
    const relativePath = path.join(".codex", "hooks", hook);
    fs.mkdirSync(path.join(root, ".codex", "hooks"), { recursive: true });
    fs.copyFileSync(
      path.join(repositoryRoot, relativePath),
      path.join(root, relativePath),
    );
  }
  if (ready) {
    const interpreter =
      process.platform === "win32"
        ? ".venv/Scripts/python.exe"
        : ".venv/bin/python";
    writeFixture(root, interpreter, "fixture interpreter\n");
    writeFixture(root, ".venv/pyvenv.cfg", "version_info = 3.14.7\n");
  }
  return root;
}

function workflowInput(root: string): HookInput {
  return {
    cwd: root,
    hook_event_name: "UserPromptSubmit",
    prompt: "FyAgent hook contract test",
  };
}

function validOutput(eventName: string): string {
  return JSON.stringify({
    hookSpecificOutput: {
      hookEventName: eventName,
      additionalContext: "contract context",
    },
  });
}

beforeAll(async () => {
  runner = (await import(
    /* @vite-ignore */ pathToFileURL(runnerPath).href
  )) as RunnerModule;
});

afterEach(() => {
  while (fixtureRoots.length > 0) {
    fs.rmSync(fixtureRoots.pop()!, { recursive: true, force: true });
  }
});

describe("Codex development hook wiring", () => {
  it("keeps the nested 15-second schema and delegates only to locked mise tasks", () => {
    const hooks = JSON.parse(
      fs.readFileSync(
        path.join(repositoryRoot, ".codex", "hooks.json"),
        "utf8",
      ),
    );
    expect(hooks).toEqual({
      hooks: {
        UserPromptSubmit: [
          {
            hooks: [
              {
                type: "command",
                command:
                  "mise run --silent --skip-tools --deny-net codex:hook:workflow-state",
                timeout: 15,
              },
            ],
          },
        ],
        SubagentStart: [
          {
            matcher: "^(?:trellis-implement|trellis-check|trellis-research)$",
            hooks: [
              {
                type: "command",
                command:
                  "mise run --silent --skip-tools --deny-net codex:hook:subagent-context",
                timeout: 15,
              },
            ],
          },
        ],
      },
    });
  });

  it("declares all hook tasks as raw, read-only task metadata", () => {
    const taskFile = fs.readFileSync(
      path.join(repositoryRoot, ".mise", "tasks", "hooks.toml"),
      "utf8",
    );
    for (const task of [
      "codex:hook:workflow-state",
      "codex:hook:subagent-context",
      "codex:hooks:check",
    ]) {
      const info = JSON.parse(
        execFileSync("mise", ["tasks", "info", "--json", task], {
          cwd: repositoryRoot,
          encoding: "utf8",
        }),
      );
      expect(info.raw, task).toBe(true);
      expect(info.env, task).toContain("FYAGENT_TASK_EFFECT=read-only");
    }
    expect(taskFile.match(/raw = true/g)).toHaveLength(3);
    expect(taskFile.match(/FYAGENT_TASK_EFFECT = "read-only"/g)).toHaveLength(
      3,
    );
  });

  it("preserves one raw JSON stdin/stdout protocol through mise", () => {
    const input = JSON.stringify({
      cwd: repositoryRoot,
      hook_event_name: "UserPromptSubmit",
      prompt: "FyAgent raw hook protocol probe",
    });
    const result = spawnSync(
      "mise",
      [
        "run",
        "--silent",
        "--skip-tools",
        "--deny-net",
        "codex:hook:workflow-state",
      ],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: {
          ...process.env,
          TRELLIS_HOOKS: "1",
          TRELLIS_DISABLE_HOOKS: "0",
        },
        input,
        timeout: 20_000,
      },
    );

    expect(result.status, result.stderr).toBe(0);
    const output = JSON.parse(result.stdout);
    expect(output.continue ?? true).toBe(true);
    expect(output.hookSpecificOutput.hookEventName).toBe("UserPromptSubmit");
  });
});

describe("Codex hook runner contract", () => {
  it("uses the exact locked, no-sync, offline uv invocation without side effects", () => {
    const root = createHookFixture({ ready: true });
    const before = runner.snapshotTree(root);
    const calls: Array<{
      command: string;
      args: string[];
      options: Record<string, unknown>;
    }> = [];
    const spawn: SpawnStub = (command, args, options) => {
      calls.push({ command, args, options });
      return {
        status: 0,
        signal: null,
        stderr: "",
        stdout: validOutput("UserPromptSubmit"),
      };
    };

    const output = runner.executeHook({
      projectRoot: root,
      mode: "workflow-state",
      input: workflowInput(root),
      spawn,
    });

    expect(output.hookSpecificOutput).toMatchObject({
      hookEventName: "UserPromptSubmit",
      additionalContext: "contract context",
    });
    expect(calls).toHaveLength(1);
    expect(calls[0].command).toBe("uv");
    expect(calls[0].args).toEqual([
      "run",
      "--locked",
      "--no-sync",
      "--offline",
      "python",
      "-X",
      "utf8",
      ".codex/hooks/inject-workflow-state.py",
    ]);
    expect(calls[0].options).toMatchObject({
      cwd: root,
      timeout: 12_000,
    });
    expect(calls[0].options.env).toMatchObject({
      FYAGENT_CODEX_HOOK_STRICT: "1",
      PYTHONDONTWRITEBYTECODE: "1",
      UV_NO_SYNC: "1",
      UV_OFFLINE: "1",
    });
    expect(runner.snapshotTree(root)).toEqual(before);
  });

  it("continues visibly when the valid project has not prepared .venv", () => {
    const root = createHookFixture();
    const before = runner.snapshotTree(root);
    const output = runner.executeHook({
      projectRoot: root,
      mode: "workflow-state",
      input: workflowInput(root),
      spawn: () => {
        throw new Error("uv must not run before bootstrap");
      },
    });

    expect(output.continue).toBe(true);
    expect(output.systemMessage).toMatch(/mise run bootstrap/);
    expect(output.hookSpecificOutput).toMatchObject({
      hookEventName: "UserPromptSubmit",
    });
    expect(runner.snapshotTree(root)).toEqual(before);
  });

  it("rejects cross-drive and symlink-escaped cwd values plus a symlinked project .venv", () => {
    expect(
      runner.isPathInsideProject(
        "C:\\projects\\fyagent",
        "C:\\projects\\fyagent\\src",
        path.win32,
      ),
    ).toBe(true);
    expect(
      runner.isPathInsideProject(
        "C:\\projects\\fyagent",
        "D:\\outside\\workspace",
        path.win32,
      ),
    ).toBe(false);

    const root = createHookFixture();
    const outside = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-hooks-outside-test-"),
    );
    fixtureRoots.push(outside);
    const escapedCwd = path.join(root, "escaped-cwd");
    fs.symlinkSync(
      outside,
      escapedCwd,
      process.platform === "win32" ? "junction" : "dir",
    );
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: { ...workflowInput(root), cwd: escapedCwd },
      }),
    ).toThrow(/cwd must remain inside/);

    const linkedVenv = path.join(root, "linked-venv-target");
    fs.mkdirSync(linkedVenv, { recursive: true });
    fs.symlinkSync(
      linkedVenv,
      path.join(root, ".venv"),
      process.platform === "win32" ? "junction" : "dir",
    );

    expect(() => runner.validateProject(root, "workflow-state")).toThrow(
      /repository-local directory, not a symlink/,
    );
  });

  it("fails closed for damaged project files and hook scripts before degradation", () => {
    const damagedLock = createHookFixture();
    writeFixture(damagedLock, "uv.lock", "version = 1\nrevision = 3\n");
    expect(() => runner.validateProject(damagedLock, "workflow-state")).toThrow(
      /requires-python/,
    );

    const damagedHook = createHookFixture();
    fs.appendFileSync(
      path.join(damagedHook, ".codex/hooks/inject-workflow-state.py"),
      "# damaged\n",
    );
    expect(() => runner.validateProject(damagedHook, "workflow-state")).toThrow(
      /integrity check failed/,
    );
  });

  it("binds required TOML keys to their approved tables and values", () => {
    const wrongPyprojectTable = createHookFixture();
    const wrongPyproject = fs
      .readFileSync(path.join(wrongPyprojectTable, "pyproject.toml"), "utf8")
      .replace("[tool.uv]", "[tool.uv]\n\n[tool.fake]");
    writeFixture(wrongPyprojectTable, "pyproject.toml", wrongPyproject);
    expect(() =>
      runner.validateProject(wrongPyprojectTable, "workflow-state"),
    ).toThrow(/package exactly once in \[tool\.uv\]/);

    const duplicatePyprojectKey = createHookFixture();
    fs.appendFileSync(
      path.join(duplicatePyprojectKey, "pyproject.toml"),
      "package = false\n",
    );
    expect(() =>
      runner.validateProject(duplicatePyprojectKey, "workflow-state"),
    ).toThrow(/package exactly once in \[tool\.uv\].*found 2/);

    const invalidPyprojectValue = createHookFixture();
    const invalidPyproject = fs
      .readFileSync(path.join(invalidPyprojectValue, "pyproject.toml"), "utf8")
      .replace('python-downloads = "automatic"', 'python-downloads = "never"');
    writeFixture(invalidPyprojectValue, "pyproject.toml", invalidPyproject);
    expect(() =>
      runner.validateProject(invalidPyprojectValue, "workflow-state"),
    ).toThrow(/python-downloads must be "automatic"/);

    const wrongLockTable = createHookFixture();
    const wrongLock = fs
      .readFileSync(path.join(wrongLockTable, "uv.lock"), "utf8")
      .replace(
        'requires-python = "==3.14.*"',
        '[tool.fake]\nrequires-python = "==3.14.*"',
      );
    writeFixture(wrongLockTable, "uv.lock", wrongLock);
    expect(() =>
      runner.validateProject(wrongLockTable, "workflow-state"),
    ).toThrow(/requires-python exactly once in the top level/);

    const invalidLockValue = createHookFixture();
    const invalidLock = fs
      .readFileSync(path.join(invalidLockValue, "uv.lock"), "utf8")
      .replace("revision = 3", "revision = 4");
    writeFixture(invalidLockValue, "uv.lock", invalidLock);
    expect(() =>
      runner.validateProject(invalidLockValue, "workflow-state"),
    ).toThrow(/revision must be 3/);
  });

  it("keeps reviewed hook integrity stable across LF and CRLF checkouts", () => {
    const root = createHookFixture();
    for (const hook of [
      "inject-workflow-state.py",
      "inject-subagent-context.py",
    ]) {
      const hookPath = path.join(root, ".codex", "hooks", hook);
      const source = fs.readFileSync(hookPath, "utf8");
      fs.writeFileSync(hookPath, source.replace(/\n/g, "\r\n"), "utf8");
    }

    expect(runner.validateProject(root, "workflow-state")).toMatchObject({
      ready: false,
    });
    expect(runner.validateProject(root, "subagent-context")).toMatchObject({
      ready: false,
    });
  });

  it("fails closed for malformed input, wrong events, nonzero hooks, and invalid stdout", () => {
    expect(() => runner.parseInput("{", "workflow-state")).toThrow(
      /valid JSON/,
    );
    expect(() =>
      runner.parseInput(
        JSON.stringify({ hook_event_name: "SubagentStart" }),
        "workflow-state",
      ),
    ).toThrow(/requires event UserPromptSubmit/);
    expect(() =>
      runner.parseInput(
        JSON.stringify({
          hook_event_name: "SubagentStart",
          agent_type: "trellis-check",
        }),
        "subagent-context",
      ),
    ).toThrow(/session_id/);

    const root = createHookFixture({ ready: true });
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: workflowInput(root),
        spawn: () => ({
          status: 2,
          signal: null,
          stderr: "broken hook",
          stdout: "",
        }),
      }),
    ).toThrow(/exited 2/);
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: workflowInput(root),
        spawn: () => ({
          status: 0,
          signal: null,
          stderr: "",
          stdout: "not-json",
        }),
      }),
    ).toThrow(/exactly one JSON object/);
  });

  it("allows only explicit Trellis disablement to be silent", () => {
    expect(runner.explicitDisable({ TRELLIS_HOOKS: "0" })).toBe(true);
    expect(runner.explicitDisable({ TRELLIS_DISABLE_HOOKS: "1" })).toBe(true);
    expect(runner.explicitDisable({})).toBe(false);

    const disabled = spawnSync(
      process.execPath,
      [runnerPath, "workflow-state"],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: { ...process.env, TRELLIS_HOOKS: "0" },
        input: "not-json",
      },
    );
    expect(disabled.status, disabled.stderr).toBe(0);
    expect(disabled.stdout).toBe("");

    const invalid = spawnSync(
      process.execPath,
      [runnerPath, "workflow-state"],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: {
          ...process.env,
          TRELLIS_HOOKS: "1",
          TRELLIS_DISABLE_HOOKS: "0",
        },
        input: "not-json",
      },
    );
    expect(invalid.status).toBe(1);
    expect(invalid.stderr).toMatch(/stdin must be valid JSON/);
  });

  it("keeps Codex Python protocol errors strict without changing generic fallbacks", () => {
    const workflowSource = fs.readFileSync(
      path.join(repositoryRoot, ".codex/hooks/inject-workflow-state.py"),
      "utf8",
    );
    const subagentSource = fs.readFileSync(
      path.join(repositoryRoot, ".codex/hooks/inject-subagent-context.py"),
      "utf8",
    );
    expect(workflowSource).toMatch(
      /FYAGENT_CODEX_HOOK_STRICT[\s\S]*raise TimeoutError/,
    );
    expect(subagentSource).toMatch(
      /print\(json\.dumps\(\{"continue": True\}\)\)/,
    );

    const python =
      process.platform === "win32"
        ? path.join(repositoryRoot, ".venv", "Scripts", "python.exe")
        : path.join(repositoryRoot, ".venv", "bin", "python");
    const script = path.join(
      repositoryRoot,
      ".codex",
      "hooks",
      "inject-subagent-context.py",
    );
    const input = JSON.stringify({
      hook_event_name: "SubagentStart",
      agent_type: "trellis-check",
      session_id: "failure-policy-test",
      cwd: repositoryRoot,
    });
    const harness = [
      "import importlib.util, io, sys",
      `spec = importlib.util.spec_from_file_location("fyagent_hook", ${JSON.stringify(script)})`,
      "module = importlib.util.module_from_spec(spec)",
      "spec.loader.exec_module(module)",
      'module._handle_codex_subagent_start = lambda _input: (_ for _ in ()).throw(RuntimeError("injected failure"))',
      `sys.stdin = io.StringIO(${JSON.stringify(input)})`,
      "module.main()",
    ].join("\n");

    const generic = spawnSync(python, ["-X", "utf8", "-c", harness], {
      cwd: repositoryRoot,
      encoding: "utf8",
      env: { ...process.env, FYAGENT_CODEX_HOOK_STRICT: "0" },
    });
    expect(generic.status, generic.stderr).toBe(0);
    expect(JSON.parse(generic.stdout)).toEqual({ continue: true });

    const strict = spawnSync(python, ["-X", "utf8", "-c", harness], {
      cwd: repositoryRoot,
      encoding: "utf8",
      env: { ...process.env, FYAGENT_CODEX_HOOK_STRICT: "1" },
    });
    expect(strict.status).not.toBe(0);
    expect(strict.stderr).toContain("injected failure");
  });
});

describe("hook command side-effect boundary", () => {
  it("contains no sync, install, trust, or warning-suppression escape hatch", () => {
    const source = fs.readFileSync(runnerPath, "utf8");
    expect(source).toContain('"--locked"');
    expect(source).toContain('"--no-sync"');
    expect(source).toContain('"--offline"');
    expect(source).not.toMatch(/\buv\s+sync\b/);
    expect(source).not.toMatch(/\bpip\s+install\b/);
    expect(source).not.toMatch(/\bmise\s+trust\b/);
    expect(source).not.toMatch(/NODE_NO_WARNINGS|--no-warnings/);
  });
});
