import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const source = fs
  .readFileSync(path.join(ROOT, ".github", "workflows", "ci.yml"), "utf8")
  .replace(/\r\n/g, "\n");

const REQUIRED_JOBS = [
  "contracts",
  "frontend",
  "desktop-acceptance-contract",
  "backend-linux",
  "backend-windows",
  "windows-msi-query",
  "backend-macos",
] as const;

const LOCAL_MISE_TESTS = [
  "tests/developmentEnvironment.test.ts",
  "tests/developmentHooks.test.ts",
  "tests/miseTaskContract.test.ts",
  "tests/systemCheck.test.ts",
  "tests/taskDocs.test.ts",
] as const;

const ACTION_PINS = new Map([
  ["actions/checkout", ["3d3c42e5aac5ba805825da76410c181273ba90b1", "v7.0.1"]],
  [
    "actions/setup-node",
    ["820762786026740c76f36085b0efc47a31fe5020", "v7.0.0"],
  ],
  [
    "pnpm/action-setup",
    ["0977fd99725f1db4007ccb2928dbb4e90d06cc86", "v6.0.10"],
  ],
  [
    "actions-rust-lang/setup-rust-toolchain",
    ["166cdcfd11aee3cb47222f9ddb555ce30ddb9659", "v1.17.0"],
  ],
  [
    "astral-sh/setup-uv",
    ["c771a70e6277c0a99b617c7a806ffedaca235ff9", "v9.0.0"],
  ],
]);

function jobBlock(id: string): string {
  const jobsStart = source.indexOf("\njobs:\n");
  const header = `\n  ${id}:\n`;
  const start = source.indexOf(header, jobsStart);
  expect(start, id).toBeGreaterThan(-1);
  const bodyStart = start + header.length;
  const next = source.slice(bodyStart).search(/^  [a-z][a-z0-9-]*:\s*$/m);
  if (next < 0) return source.slice(start + 1);
  return source.slice(start + 1, bodyStart + next);
}

function actionSteps(action: string): string[] {
  const escaped = action.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const expression = new RegExp(
    `^      - name: [^\\n]+\\n(?:        [^\\n]+\\n)*?        uses: ${escaped}@[a-f0-9]{40}[^\\n]*\\n(?:        [^\\n]+\\n|          [^\\n]+\\n)*`,
    "gm",
  );
  return source.match(expression) ?? [];
}

describe("automatic CI workflow", () => {
  it("keeps exactly seven unconditional dependencies behind one stable gate", () => {
    const jobsSection = source.slice(source.indexOf("\njobs:\n"));
    const jobIds = [...jobsSection.matchAll(/^  ([a-z][a-z0-9-]*):\s*$/gm)].map(
      (match) => match[1],
    );
    expect(jobIds).toEqual([...REQUIRED_JOBS, "required"]);

    for (const id of REQUIRED_JOBS) {
      expect(jobBlock(id), id).not.toMatch(/^    if:/m);
    }

    const required = jobBlock("required");
    expect(required).toContain("name: CI / Required");
    expect(required).toContain("if: always()");
    expect(required).toContain("runs-on: ubuntu-24.04");
    expect(required).toContain("REQUIRED_RESULTS: ${{ toJSON(needs) }}");
    expect(required).toContain("run: node scripts/ci/required-gate.mjs");
    for (const id of REQUIRED_JOBS) {
      expect(required).toContain(`      - ${id}`);
    }
  });

  it("uses only explicit approved runners and read-only workflow permissions", () => {
    expect(source).toContain("permissions:\n  contents: read");
    expect(source).not.toMatch(/^\s+[a-z][a-z-]*:\s+write\s*$/m);
    expect(source).not.toContain("secrets.");
    expect(source).not.toMatch(/runs-on:\s*[^\n]*-latest/);
    expect(source).not.toMatch(/(?:^|\s)paths(?:-ignore)?:/m);

    expect(jobBlock("contracts")).toContain("runs-on: ubuntu-24.04");
    expect(jobBlock("frontend")).toContain("runs-on: ubuntu-24.04");
    expect(jobBlock("desktop-acceptance-contract")).toContain(
      "runs-on: ubuntu-24.04",
    );
    expect(jobBlock("backend-linux")).toContain("runs-on: ubuntu-24.04");
    expect(jobBlock("backend-windows")).toContain("runs-on: windows-2022");
    expect(jobBlock("windows-msi-query")).toContain(
      "runs-on: ${{ matrix.runner }}",
    );
    expect(jobBlock("backend-macos")).toContain("runs-on: macos-15");
  });

  it("pins every third-party Action to an reviewed full commit", () => {
    const uses = [...source.matchAll(/^\s+uses:\s+(.+)$/gm)].map(
      (match) => match[1],
    );
    expect(uses.length).toBeGreaterThan(0);
    const parsedUses = uses.map((reference) => {
      const parsed = /^([^@\s]+)@([a-f0-9]{40})\s+#\s+(\S+)$/.exec(reference);
      expect(parsed, reference).not.toBeNull();
      return parsed!;
    });
    for (const [, action, sha, version] of parsedUses) {
      expect(ACTION_PINS.has(action), action).toBe(true);
      expect([sha, version], action).toEqual(ACTION_PINS.get(action));
      expect(sha).toMatch(/^[a-f0-9]{40}$/);
    }
    for (const action of ACTION_PINS.keys()) {
      expect(
        parsedUses.some((match) => match[1] === action),
        action,
      ).toBe(true);
    }

    const checkoutSteps = actionSteps("actions/checkout");
    expect(checkoutSteps).toHaveLength(8);
    for (const step of checkoutSteps) {
      expect(step).toContain("persist-credentials: false");
    }
  });

  it("consumes standard version files without mise or duplicate literals", () => {
    expect(source).not.toMatch(/\bmise\s+(?:run|exec|install|trust)\b/);
    expect(source).not.toContain("24.19.0");
    expect(source).not.toContain("10.12.3");
    expect(source).not.toContain("1.97.1");
    expect(source).not.toContain("0.12.2");
    expect(source).not.toContain("3.14.7");

    const nodeSteps = actionSteps("actions/setup-node");
    expect(nodeSteps).toHaveLength(7);
    for (const step of nodeSteps) {
      expect(step).toContain("node-version-file: .node-version");
      expect(step).not.toMatch(/^\s+node-version:/m);
    }

    const pnpmSteps = actionSteps("pnpm/action-setup");
    expect(pnpmSteps).toHaveLength(6);
    for (const step of pnpmSteps) {
      expect(step).toContain("run_install: false");
      expect(step).not.toMatch(/^\s+version:/m);
    }

    const rustSteps = actionSteps("actions-rust-lang/setup-rust-toolchain");
    expect(rustSteps).toHaveLength(4);
    for (const step of rustSteps) {
      expect(step).toContain("cache: false");
      expect(step).toContain('rustflags: ""');
      expect(step).not.toMatch(/^\s+(?:toolchain|components?|targets?):/m);
    }

    const uvSteps = actionSteps("astral-sh/setup-uv");
    expect(uvSteps).toHaveLength(3);
    for (const step of uvSteps) {
      expect(step).toContain(
        "version: ${{ steps.toolchain-facts.outputs.uv-version }}",
      );
      expect(step).toContain("enable-cache: false");
    }
    expect(
      uvSteps.filter((step) =>
        step.includes(
          "python-version: ${{ steps.toolchain-facts.outputs.python-version }}",
        ),
      ),
    ).toHaveLength(2);
    expect(uvSteps).toContainEqual(
      expect.stringContaining(
        "python-version: cpython-${{ steps.toolchain-facts.outputs.python-version }}-windows-${{ matrix.python_architecture }}-none",
      ),
    );
  });

  it("prepares the locked Python environment before the full unit suite", () => {
    const contracts = jobBlock("contracts");
    const frontend = jobBlock("frontend");
    for (const block of [contracts, frontend]) {
      expect(block).toContain(
        "run: node scripts/ci/verify-toolchain.mjs --emit-github-output",
      );
      expect(block).toContain("run: uv sync --locked");
      expect(block.indexOf("run: uv sync --locked")).toBeLessThan(
        block.indexOf("node scripts/tasks/release-check.mjs --ci") >= 0
          ? block.indexOf("node scripts/tasks/release-check.mjs --ci")
          : block.indexOf("pnpm test:unit"),
      );
    }
    expect(frontend).toContain("pnpm test:unit");
    expect(frontend).not.toContain("NODE_OPTIONS:");
  });

  it("keeps GitHub Actions independent from the local mise runtime", () => {
    const contracts = jobBlock("contracts");
    const frontend = jobBlock("frontend");
    const releaseCheck = fs.readFileSync(
      path.join(ROOT, "scripts", "tasks", "release-check.mjs"),
      "utf8",
    );

    expect(contracts).toContain(
      "run: node scripts/tasks/release-check.mjs --ci",
    );
    expect(frontend).toContain("pnpm test:unit");
    expect(frontend).not.toContain("run: pnpm test:unit");
    for (const test of LOCAL_MISE_TESTS) {
      expect(frontend, test).toContain(`--exclude ${test}`);
      expect(releaseCheck, test).toContain(`\"${test}\"`);
    }
    expect(frontend).toContain("pnpm test:unit tests/developmentHooks.test.ts");
    expect(frontend).toMatch(
      /declares all hook tasks as raw, read-only task\s+metadata/,
    );
    expect(frontend).toMatch(
      /preserves one raw JSON\s+stdin\/stdout protocol through mise/,
    );
    expect(releaseCheck).toContain("if (!ciMode)");
    expect(releaseCheck).toContain(
      'run("node", ["scripts/tasks/task-contract-check.mjs"]);',
    );
  });

  it("runs locked Rust checks on Linux, Windows, and macOS", () => {
    for (const id of ["backend-linux", "backend-windows", "backend-macos"]) {
      const block = jobBlock(id);
      expect(block).toContain(
        "node scripts/ci/verify-toolchain.mjs --tools node,pnpm,rust",
      );
      expect(block).toContain(
        "cargo check --workspace --all-targets --locked --manifest-path src-tauri/Cargo.toml",
      );
      expect(block).toContain(
        "cargo clippy --workspace --all-targets --locked --manifest-path src-tauri/Cargo.toml -- -D warnings",
      );
      expect(block).toContain(
        "cargo test --workspace --locked --manifest-path src-tauri/Cargo.toml",
      );
    }
    expect(jobBlock("backend-windows")).toContain(
      "FYAGENT_WINDOWS_MANIFEST: test",
    );
    expect(jobBlock("backend-linux")).toContain(
      "cargo fmt --all --check --manifest-path src-tauri/Cargo.toml",
    );
  });

  it("runs managed Python, Trellis, and the query fixture on native Windows x64 and ARM64", () => {
    const block = jobBlock("windows-msi-query");
    expect(block).toContain(
      "name: Windows Native Contracts (${{ matrix.architecture }})",
    );
    expect(block).toContain("timeout-minutes: 15");
    expect(block).toContain("fail-fast: false");
    expect(block).toContain(`matrix:
        include:
          - runner: windows-2022
            architecture: X64
            python_architecture: x86_64
            python_platform: win-amd64
          - runner: windows-11-arm
            architecture: Arm64
            python_architecture: aarch64
            python_platform: win-arm64
    runs-on: \${{ matrix.runner }}`);
    expect(block).toContain('UV_MANAGED_PYTHON: "true"');
    expect(block).toContain("shell: pwsh");
    expect(block).toContain("node-version-file: .node-version");
    expect(block).toContain(
      "run: node scripts/ci/verify-toolchain.mjs --emit-github-output",
    );
    expect(block).toContain(
      "version: ${{ steps.toolchain-facts.outputs.uv-version }}",
    );
    expect(block).toContain(
      "python-version: cpython-${{ steps.toolchain-facts.outputs.python-version }}-windows-${{ matrix.python_architecture }}-none",
    );
    expect(block).toContain("enable-cache: false");
    expect(block).toContain("run: uv sync --locked --managed-python");
    expect(block).toContain(
      "run: node scripts/ci/verify-toolchain.mjs --tools node,uv,python",
    );
    expect(block).toContain(
      "uv run --locked --no-sync python .trellis/scripts/task.py list --json",
    );
    expect(block).toContain(
      'python -c "import sysconfig; print(sysconfig.get_platform())"',
    );
    expect(block).toContain(
      "$pythonPlatform -ne '${{ matrix.python_platform }}'",
    );
    expect(block).toContain(
      "-not ($parsed.PSObject.Properties.Name -contains 'tasks')",
    );
    expect(block).toContain("-not ($parsed.tasks -is [System.Array])");
    expect(block).toContain(
      'throw "Trellis task listing did not return a tasks array"',
    );
    expect(block).toContain("./tests/windowsInstallerQuery.integration.ps1");
    expect(block).toContain(
      "-ExpectedArchitecture '${{ matrix.architecture }}'",
    );
    expect(block.match(/^      - name:/gm)).toHaveLength(8);
    expect(block.match(/^        uses:/gm)).toHaveLength(3);
    expect(block).not.toMatch(
      /setup-rust|\b(?:npm|npx|pnpm|yarn|bun|cargo|rustc)\b|tauri|src-tauri|frontend|package|bundle/i,
    );
  });

  it("runs the focused pending-deprecation native Fetch probe", () => {
    const desktop = jobBlock("desktop-acceptance-contract");
    expect(desktop).toContain("run: pnpm test:native-fetch");
    expect(source).not.toContain("NODE_NO_WARNINGS");
    expect(source).not.toContain("--no-warnings");
    expect(source).not.toContain("--no-deprecation");
    expect(source).not.toContain("--disable-warning=DEP0040");
  });
});
