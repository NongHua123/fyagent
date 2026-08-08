import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error The contract executes this JavaScript helper directly.
import * as dep0040Module from "../scripts/tasks/dep0040-check.mjs";

const {
  analyzeWhyGraph,
  extractModuleSpecifiers,
  findSuppressionViolations,
  parsePnpmLock,
  reconcileLockAndWhy,
  validateActiveModuleSpecifiers,
  validateManifest,
  validateNoSuppression,
  validateRuntime,
} = dep0040Module;

const ROOT = path.resolve(__dirname, "..");
const read = (relative: string) =>
  fs.readFileSync(path.join(ROOT, relative), "utf8").replace(/\r\n/g, "\n");

describe("DEP0040 dependency and deprecation contract", () => {
  it("requires the pinned Node runtime and its unmarked native Web APIs", () => {
    expect(validateRuntime(ROOT)).toMatchObject({
      expected: "24.19.0",
      actual: "24.19.0",
      globals: ["fetch", "Headers", "Request", "Response"],
    });

    const setup = read("tests/setupGlobals.ts");
    expect(setup).toContain('path.resolve(process.cwd(), ".node-version")');
    expect(setup).toContain("process.versions.node !== expectedNodeVersion");
    for (const name of ["fetch", "Headers", "Request", "Response"]) {
      expect(setup).toContain(`"${name}"`);
    }
    expect(setup).toContain(
      'Object.prototype.hasOwnProperty.call(globalThis.fetch, "polyfill")',
    );
  });

  it("removes direct compatibility dependencies and active module imports", () => {
    expect(validateManifest(ROOT)).toMatchObject({
      forbiddenDependencies: [],
    });
    expect(validateActiveModuleSpecifiers(ROOT)).toMatchObject({
      files: expect.any(Number),
      specifiers: expect.any(Number),
    });
    expect(read("tests/msw/tauriMocks.ts")).not.toContain(
      'import "cross-fetch/polyfill"',
    );

    expect(
      extractModuleSpecifiers(
        [
          'import nativeValue from "node:fs";',
          "const negativeFixture = 'import \"cross-fetch/polyfill\";';",
          "// import 'node-fetch';",
          "void nativeValue; void negativeFixture;",
        ].join("\n"),
        "fixture.ts",
      ),
    ).toEqual(["node:fs"]);
    expect(
      extractModuleSpecifiers('import "cross-fetch/polyfill";', "bad.ts"),
    ).toEqual(["cross-fetch/polyfill"]);
    expect(() =>
      extractModuleSpecifiers('import "cross-fetch/polyfill;', "broken.ts"),
    ).toThrow("Cannot parse active module broken.ts");
  });

  it("parses the versioned lock and permits only the modern jsdom URL path", () => {
    const lock = parsePnpmLock(read("pnpm-lock.yaml"));
    expect(lock.lockfileVersion).toBe("9.0");
    expect(lock.packages).toEqual(
      expect.arrayContaining([
        { name: "punycode", version: "2.3.1" },
        { name: "tr46", version: "5.1.1" },
        { name: "whatwg-url", version: "14.2.0" },
      ]),
    );

    const obsoleteLock = [
      "lockfileVersion: '9.0'",
      "importers:",
      "",
      "  .:",
      "    dependencies: {}",
      "",
      "packages:",
      "",
      "  node-fetch@2.7.0:",
      "    resolution: {}",
      "",
      "  whatwg-url@5.0.0:",
      "    resolution: {}",
      "",
      "  tr46@0.0.3:",
      "    resolution: {}",
      "",
      "snapshots:",
      "",
      "  node-fetch@2.7.0:",
      "    dependencies: {}",
      "",
      "  whatwg-url@5.0.0:",
      "    dependencies: {}",
      "",
      "  tr46@0.0.3:",
      "    dependencies: {}",
      "",
    ].join("\n");
    expect(() => parsePnpmLock(obsoleteLock)).toThrow(
      "Obsolete DEP0040 lock entries remain",
    );

    const mismatchedLock = obsoleteLock
      .replace(/\n  node-fetch@2\.7\.0:\n    resolution: \{\}\n/, "\n")
      .replace(/\n  whatwg-url@5\.0\.0:\n    resolution: \{\}\n/, "\n")
      .replace(/\n  tr46@0\.0\.3:\n    resolution: \{\}\n/, "\n");
    expect(() => parsePnpmLock(mismatchedLock)).toThrow(
      "Watched pnpm package/snapshot mismatch",
    );

    expect(() =>
      parsePnpmLock(
        read("pnpm-lock.yaml").replace(
          "  punycode@2.3.1:",
          "  punycode@2.3.1: {}",
        ),
      ),
    ).toThrow("Unsupported pnpm packages entry format");
  });

  it("constructs reverse paths and rejects the historical dependency chain", () => {
    const allowed = analyzeWhyGraph([
      {
        name: "fyagent",
        devDependencies: {
          jsdom: {
            from: "jsdom",
            version: "25.0.1",
            dependencies: {
              "whatwg-url": {
                from: "whatwg-url",
                version: "14.2.0",
                dependencies: {
                  tr46: {
                    from: "tr46",
                    version: "5.1.1",
                    dependencies: {
                      punycode: { from: "punycode", version: "2.3.1" },
                    },
                  },
                },
              },
            },
          },
        },
      },
    ]);
    expect(allowed.map((entry: { path: string }) => entry.path)).toContain(
      "fyagent -> jsdom@25.0.1 -> whatwg-url@14.2.0 -> tr46@5.1.1 -> punycode@2.3.1",
    );
    expect(
      reconcileLockAndWhy(
        [
          { name: "whatwg-url", version: "14.2.0" },
          { name: "tr46", version: "5.1.1" },
          { name: "punycode", version: "2.3.1" },
        ],
        allowed,
      ).explainedPackages,
    ).toEqual(["punycode@2.3.1", "tr46@5.1.1", "whatwg-url@14.2.0"]);

    const obsolete = [
      {
        name: "fyagent",
        devDependencies: {
          "cross-fetch": {
            from: "cross-fetch",
            version: "4.1.0",
            dependencies: {
              "node-fetch": { from: "node-fetch", version: "2.7.0" },
            },
          },
        },
      },
    ];
    expect(() => analyzeWhyGraph(obsolete)).toThrow(
      "Obsolete DEP0040 reverse paths remain",
    );

    expect(() =>
      analyzeWhyGraph([
        {
          name: "fyagent",
          devDependencies: {
            unrelated: {
              from: "unrelated",
              version: "1.0.0",
              dependencies: {
                punycode: { from: "punycode", version: "2.3.1" },
              },
            },
          },
        },
      ]),
    ).toThrow("Unexpected watched dependency paths remain");

    expect(() =>
      analyzeWhyGraph([
        {
          name: "fyagent",
          devDependencies: {
            alias: { from: "jsdom", version: "25.0.1" },
          },
        },
      ]),
    ).toThrow("pnpm why alias is not reviewed");
  });

  it("uses portable deprecation flags without warning suppression", () => {
    const manifest = JSON.parse(read("package.json")) as {
      scripts: Record<string, string>;
    };
    for (const name of [
      "test:unit",
      "test:unit:watch",
      "test:i18n",
      "test:desktop:mock",
      "test:desktop:visual:preflight",
      "test:desktop:visual:update",
    ]) {
      expect(manifest.scripts[name], name).toContain("--throw-deprecation");
    }
    expect(manifest.scripts["test:native-fetch"]).toContain(
      "--pending-deprecation --throw-deprecation",
    );
    expect(validateNoSuppression(ROOT)).toMatchObject({ violations: [] });
    expect(
      findSuppressionViolations([
        {
          file: "negative-fixture",
          source: "node --no-warnings ./node_modules/vitest/vitest.mjs run",
        },
        {
          file: "stderr-filter-fixture",
          source: "node probe.mjs 2>&1 | grep -v DEP0040",
        },
        {
          file: "concatenated-fixture.mjs",
          source: 'const hidden = "--no" + "-deprecation";',
        },
        {
          file: "joined-fixture.mjs",
          source:
            'const parts = ["NODE_NO_", "WARNINGS"]; const hidden = parts.join("");',
        },
      ]),
    ).toEqual([
      "negative-fixture: --no-warnings",
      "stderr-filter-fixture: Node stderr filtering",
      "stderr-filter-fixture: deprecation stderr filtering",
      "concatenated-fixture.mjs: --no-deprecation",
      "joined-fixture.mjs: NODE_NO_WARNINGS",
    ]);
  });

  it("reuses the task runner's native pnpm resolver without a batch shim", () => {
    const source = read("scripts/tasks/dep0040-check.mjs");
    expect(source).toContain(
      'import { resolveTaskExecutable } from "./lib.mjs";',
    );
    expect(source).toContain(
      'spawnSync(resolveTaskExecutable("pnpm"), args, {',
    );
    expect(source).not.toContain("pnpm.cmd");
    expect(source).not.toContain("function pnpmExecutable");
  });
});
