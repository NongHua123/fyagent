#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";

const SCRIPT_DIRECTORY = path.dirname(fileURLToPath(import.meta.url));
export const ROOT = path.resolve(SCRIPT_DIRECTORY, "..", "..");

const FORBIDDEN_DIRECT_PACKAGES = Object.freeze([
  "cross-fetch",
  "node-fetch",
  "isomorphic-fetch",
  "whatwg-fetch",
  "undici",
]);
const WATCHED_GRAPH_PACKAGES = new Set([
  "cross-fetch",
  "node-fetch",
  "whatwg-url",
  "tr46",
  "punycode",
]);
const ACTIVE_MODULE_ROOTS = Object.freeze(["src", "tests", "scripts"]);
const ACTIVE_ROOT_MODULES = Object.freeze([
  "postcss.config.cjs",
  "tailwind.config.cjs",
  "vite.config.ts",
  "vitest.config.ts",
]);
const MODULE_EXTENSIONS = new Set([
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
  ".ts",
  ".tsx",
]);
const EXECUTABLE_SCRIPT_EXTENSIONS = new Set([
  ...MODULE_EXTENSIONS,
  ".bash",
  ".bat",
  ".cmd",
  ".ps1",
  ".py",
  ".sh",
]);

function read(root, relativePath) {
  return fs
    .readFileSync(path.join(root, relativePath), "utf8")
    .replace(/\r\n/g, "\n");
}

function packageRoot(specifier) {
  if (
    specifier.startsWith(".") ||
    specifier.startsWith("/") ||
    specifier.startsWith("node:") ||
    specifier.startsWith("#")
  ) {
    return undefined;
  }
  if (specifier.startsWith("@")) {
    return specifier.split("/").slice(0, 2).join("/");
  }
  return specifier.split("/")[0];
}

function scriptKind(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  return {
    ".js": ts.ScriptKind.JS,
    ".jsx": ts.ScriptKind.JSX,
    ".mjs": ts.ScriptKind.JS,
    ".cjs": ts.ScriptKind.JS,
    ".ts": ts.ScriptKind.TS,
    ".tsx": ts.ScriptKind.TSX,
  }[extension];
}

export function extractModuleSpecifiers(source, filePath = "source.ts") {
  const sourceFile = ts.createSourceFile(
    filePath,
    source,
    ts.ScriptTarget.Latest,
    true,
    scriptKind(filePath) ?? ts.ScriptKind.TS,
  );
  if (sourceFile.parseDiagnostics.length > 0) {
    const diagnostic = sourceFile.parseDiagnostics[0];
    throw new Error(
      `Cannot parse active module ${filePath}: ${ts.flattenDiagnosticMessageText(
        diagnostic.messageText,
        " ",
      )}`,
    );
  }
  const specifiers = [];
  const add = (literal) => {
    if (literal && ts.isStringLiteralLike(literal)) {
      specifiers.push(literal.text);
    }
  };
  const visit = (node) => {
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) {
      add(node.moduleSpecifier);
    } else if (
      ts.isImportEqualsDeclaration(node) &&
      ts.isExternalModuleReference(node.moduleReference)
    ) {
      add(node.moduleReference.expression);
    } else if (
      ts.isCallExpression(node) &&
      (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
        (ts.isIdentifier(node.expression) &&
          node.expression.text === "require"))
    ) {
      add(node.arguments[0]);
    } else if (
      ts.isImportTypeNode(node) &&
      ts.isLiteralTypeNode(node.argument)
    ) {
      add(node.argument.literal);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return specifiers;
}

function walkModuleFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (
        entry.isFile() &&
        MODULE_EXTENSIONS.has(path.extname(entry.name).toLowerCase())
      ) {
        files.push(absolute);
      }
    }
  };
  for (const relativeRoot of ACTIVE_MODULE_ROOTS) {
    const absolute = path.join(root, relativeRoot);
    if (!fs.statSync(absolute).isDirectory()) {
      throw new Error(`Active module root is missing: ${relativeRoot}`);
    }
    visit(absolute);
  }
  for (const relative of ACTIVE_ROOT_MODULES) {
    const absolute = path.join(root, relative);
    if (!fs.statSync(absolute).isFile()) {
      throw new Error(`Active root module is missing: ${relative}`);
    }
    files.push(absolute);
  }
  return files.sort();
}

export function validateActiveModuleSpecifiers(root = ROOT) {
  const violations = [];
  let specifierCount = 0;
  const files = walkModuleFiles(root);
  for (const absolute of files) {
    const relative = path.relative(root, absolute).split(path.sep).join("/");
    const specifiers = extractModuleSpecifiers(
      fs.readFileSync(absolute, "utf8"),
      relative,
    );
    specifierCount += specifiers.length;
    for (const specifier of specifiers) {
      const dependency = packageRoot(specifier);
      if (dependency && FORBIDDEN_DIRECT_PACKAGES.includes(dependency)) {
        violations.push(`${relative}: ${specifier}`);
      }
    }
  }
  if (violations.length > 0) {
    throw new Error(
      `Forbidden Fetch compatibility module specifiers remain: ${violations.join(
        "; ",
      )}`,
    );
  }
  return { files: files.length, specifiers: specifierCount };
}

export function validateManifest(root = ROOT) {
  const manifest = JSON.parse(read(root, "package.json"));
  const violations = [];
  for (const section of [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
  ]) {
    for (const dependency of Object.keys(manifest[section] ?? {})) {
      if (FORBIDDEN_DIRECT_PACKAGES.includes(dependency)) {
        violations.push(`${section}.${dependency}`);
      }
    }
  }
  if (violations.length > 0) {
    throw new Error(
      `Forbidden direct Fetch compatibility dependencies remain: ${violations.join(
        ", ",
      )}`,
    );
  }
  return {
    packageManager: manifest.packageManager,
    forbiddenDependencies: [],
  };
}

function unquoteYamlKey(raw) {
  const value = raw.trim();
  if (value.startsWith("'") && value.endsWith("'")) {
    return value.slice(1, -1).replace(/''/g, "'");
  }
  if (value.startsWith('"') && value.endsWith('"')) {
    return JSON.parse(value);
  }
  return value;
}

function packageLocator(raw) {
  const locator = unquoteYamlKey(raw).replace(/\(.+$/, "");
  const separator = locator.startsWith("@")
    ? locator.indexOf("@", locator.indexOf("/") + 1)
    : locator.lastIndexOf("@");
  if (separator <= 0 || separator === locator.length - 1) return undefined;
  return {
    name: locator.slice(0, separator),
    version: locator.slice(separator + 1),
  };
}

function forbiddenLockReason(name, version) {
  if (name === "cross-fetch") return "cross-fetch is obsolete";
  if (name === "node-fetch" && /^2(?:\.|$)/.test(version)) {
    return "node-fetch major 2 is the obsolete CommonJS Fetch path";
  }
  if (name === "whatwg-url" && /^5(?:\.|$)/.test(version)) {
    return "whatwg-url major 5 belongs to the obsolete node-fetch path";
  }
  if (name === "tr46" && version === "0.0.3") {
    return "tr46 0.0.3 belongs to the obsolete DEP0040 path";
  }
  return undefined;
}

function rootImporterDependencies(source) {
  const lines = source.split("\n");
  const importerStart = lines.findIndex((line) => line === "  .:");
  if (importerStart < 0) throw new Error("pnpm lock has no root importer");
  const dependencies = [];
  let activeSection = false;
  for (let index = importerStart + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^(?:\S|  \S)/.test(line)) break;
    if (
      /^    (?:dependencies|devDependencies|optionalDependencies|peerDependencies):$/.test(
        line,
      )
    ) {
      activeSection = true;
      continue;
    }
    if (
      /^    (?:dependencies|devDependencies|optionalDependencies|peerDependencies):\s*\{\s*\}\s*$/.test(
        line,
      )
    ) {
      activeSection = false;
      continue;
    }
    if (
      /^    (?:dependencies|devDependencies|optionalDependencies|peerDependencies):\s*\{/.test(
        line,
      )
    ) {
      throw new Error(
        "pnpm root importer dependency maps must use the canonical block format",
      );
    }
    if (/^    \S/.test(line)) {
      activeSection = false;
      continue;
    }
    const match = activeSection ? /^      (.+):$/.exec(line) : null;
    if (match) dependencies.push(unquoteYamlKey(match[1]));
    else if (activeSection && /^      \S/.test(line)) {
      throw new Error(
        `Unsupported pnpm root importer dependency entry: ${line.trim()}`,
      );
    }
  }
  return dependencies;
}

export function parsePnpmLock(source) {
  const versionMatch = /^lockfileVersion:\s*['"]?([^'"\s]+)['"]?\s*$/m.exec(
    source,
  );
  if (!versionMatch || versionMatch[1] !== "9.0") {
    throw new Error("pnpm-lock.yaml must use the expected versioned v9 format");
  }

  const directDependencies = rootImporterDependencies(source);
  const forbiddenDirect = directDependencies.filter((dependency) =>
    FORBIDDEN_DIRECT_PACKAGES.includes(dependency),
  );
  if (forbiddenDirect.length > 0) {
    throw new Error(
      `Root pnpm importer retains forbidden dependencies: ${forbiddenDirect.join(
        ", ",
      )}`,
    );
  }

  const packageSections = new Map([
    ["packages", new Map()],
    ["snapshots", new Map()],
  ]);
  const seenSections = new Set();
  let section;
  for (const line of source.split("\n")) {
    if (line === "packages:" || line === "snapshots:") {
      section = line.slice(0, -1);
      seenSections.add(section);
      continue;
    }
    if (/^\S/.test(line)) {
      section = undefined;
      continue;
    }
    if (!section) continue;
    const blockMatch = /^  (\S.*):$/.exec(line);
    const emptySnapshotMatch =
      section === "snapshots" ? /^  (\S.*):\s*\{\s*\}\s*$/.exec(line) : null;
    const match = blockMatch ?? emptySnapshotMatch;
    if (/^  \S/.test(line) && !match) {
      throw new Error(
        `Unsupported pnpm ${section} entry format: ${line.trim()}`,
      );
    }
    if (!match) continue;
    const locator = packageLocator(match[1]);
    if (!locator) {
      throw new Error(
        `Cannot parse pnpm ${section} package locator: ${match[1]}`,
      );
    }
    if (!WATCHED_GRAPH_PACKAGES.has(locator.name)) continue;
    packageSections
      .get(section)
      .set(`${locator.name}@${locator.version}`, locator);
  }

  for (const requiredSection of packageSections.keys()) {
    if (!seenSections.has(requiredSection)) {
      throw new Error(`pnpm lock is missing the ${requiredSection} section`);
    }
  }
  const packageKeys = new Set(packageSections.get("packages").keys());
  const snapshotKeys = new Set(packageSections.get("snapshots").keys());
  const packageOnly = [...packageKeys].filter(
    (entry) => !snapshotKeys.has(entry),
  );
  const snapshotOnly = [...snapshotKeys].filter(
    (entry) => !packageKeys.has(entry),
  );
  if (packageOnly.length > 0 || snapshotOnly.length > 0) {
    throw new Error(
      `Watched pnpm package/snapshot mismatch; package-only=[${packageOnly.join(
        ", ",
      )}] snapshot-only=[${snapshotOnly.join(", ")}]`,
    );
  }

  const packages = packageSections.get("packages");

  const forbidden = [...packages.values()]
    .map((entry) => ({
      ...entry,
      reason: forbiddenLockReason(entry.name, entry.version),
    }))
    .filter((entry) => entry.reason);
  if (forbidden.length > 0) {
    throw new Error(
      `Obsolete DEP0040 lock entries remain: ${forbidden
        .map((entry) => `${entry.name}@${entry.version} (${entry.reason})`)
        .join(", ")}`,
    );
  }

  return {
    lockfileVersion: versionMatch[1],
    directDependencies,
    packages: [...packages.values()].sort((left, right) =>
      `${left.name}@${left.version}`.localeCompare(
        `${right.name}@${right.version}`,
      ),
    ),
  };
}

function dependencySections(record) {
  return [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
  ]
    .map((name) => [name, record?.[name]])
    .filter(([, value]) => value !== undefined);
}

function hasOrderedAncestors(entry, requirements) {
  let cursor = -1;
  for (const [name, version] of requirements) {
    cursor = entry.ancestors.findIndex(
      (ancestor, index) =>
        index > cursor &&
        ancestor.name === name &&
        (version === undefined || version.test(ancestor.version)),
    );
    if (cursor < 0) return false;
  }
  return true;
}

function unexpectedWatchedReason(entry) {
  if (
    entry.name === "whatwg-url" &&
    /^14(?:\.|$)/.test(entry.version) &&
    hasOrderedAncestors(entry, [["jsdom", undefined]])
  ) {
    return undefined;
  }
  if (
    entry.name === "tr46" &&
    /^5(?:\.|$)/.test(entry.version) &&
    hasOrderedAncestors(entry, [
      ["jsdom", undefined],
      ["whatwg-url", /^14(?:\.|$)/],
    ])
  ) {
    return undefined;
  }
  if (
    entry.name === "punycode" &&
    /^2(?:\.|$)/.test(entry.version) &&
    hasOrderedAncestors(entry, [
      ["jsdom", undefined],
      ["whatwg-url", /^14(?:\.|$)/],
      ["tr46", /^5(?:\.|$)/],
    ])
  ) {
    return undefined;
  }
  return "watched dependency is outside the reviewed jsdom native-Fetch path";
}

export function analyzeWhyGraph(document) {
  if (!Array.isArray(document) || document.length === 0) {
    throw new Error("pnpm why JSON must be a non-empty array of project roots");
  }
  const records = [];
  const walk = (dependencies, rootName, ancestors) => {
    if (
      !dependencies ||
      typeof dependencies !== "object" ||
      Array.isArray(dependencies)
    ) {
      throw new Error(
        `Malformed pnpm why dependency map at ${[
          rootName,
          ...ancestors.map((entry) => `${entry.name}@${entry.version}`),
        ].join(" -> ")}`,
      );
    }
    for (const [requestedName, node] of Object.entries(dependencies)) {
      if (!node || typeof node !== "object" || Array.isArray(node)) {
        throw new Error(`Malformed pnpm why node for ${requestedName}`);
      }
      const name = node.from;
      const version = node.version;
      if (typeof name !== "string" || typeof version !== "string") {
        throw new Error(
          `pnpm why cannot explain ${requestedName} with a name/version`,
        );
      }
      if (requestedName !== name) {
        throw new Error(
          `pnpm why alias is not reviewed: requested ${requestedName}, received ${name}@${version}`,
        );
      }
      const current = { name, version };
      const currentAncestors = [...ancestors, current];
      const currentPath = [
        rootName,
        ...currentAncestors.map((entry) => `${entry.name}@${entry.version}`),
      ].join(" -> ");
      if (WATCHED_GRAPH_PACKAGES.has(name)) {
        records.push({
          name,
          version,
          path: currentPath,
          ancestors,
        });
      }
      for (const [, nested] of dependencySections(node)) {
        walk(nested, rootName, currentAncestors);
      }
    }
  };

  for (const project of document) {
    if (!project || typeof project !== "object" || Array.isArray(project)) {
      throw new Error("pnpm why project root is malformed");
    }
    const rootName =
      typeof project.name === "string" && project.name.length > 0
        ? project.name
        : "<project>";
    for (const [, dependencies] of dependencySections(project)) {
      walk(dependencies, rootName, []);
    }
  }

  const forbidden = records
    .map((entry) => ({
      ...entry,
      reason: forbiddenLockReason(entry.name, entry.version),
    }))
    .filter((entry) => entry.reason);
  if (forbidden.length > 0) {
    throw new Error(
      `Obsolete DEP0040 reverse paths remain: ${forbidden
        .map((entry) => `${entry.path} (${entry.reason})`)
        .join("; ")}`,
    );
  }
  const unexpected = records
    .map((entry) => ({ ...entry, reason: unexpectedWatchedReason(entry) }))
    .filter((entry) => entry.reason);
  if (unexpected.length > 0) {
    throw new Error(
      `Unexpected watched dependency paths remain: ${unexpected
        .map((entry) => `${entry.path} (${entry.reason})`)
        .join("; ")}`,
    );
  }
  return records.map(({ ancestors: _ancestors, ...entry }) => entry);
}

function pnpmExecutable() {
  return process.platform === "win32" ? "pnpm.cmd" : "pnpm";
}

function spawnPnpm(root, args) {
  const result = spawnSync(pnpmExecutable(), args, {
    cwd: root,
    env: process.env,
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `pnpm ${args.join(" ")} exited with ${result.status}: ${(
        result.stderr ||
        result.stdout ||
        "no diagnostic"
      ).trim()}`,
    );
  }
  return result.stdout.trim();
}

export function readWhyGraph(root = ROOT) {
  const manifest = JSON.parse(read(root, "package.json"));
  const packageManager = /^pnpm@(\d+\.\d+\.\d+)$/.exec(
    manifest.packageManager ?? "",
  );
  if (!packageManager) {
    throw new Error(
      "package.json packageManager must pin an exact pnpm version",
    );
  }
  const actualPnpm = spawnPnpm(root, ["--version"]);
  if (actualPnpm !== packageManager[1]) {
    throw new Error(
      `pnpm version mismatch: expected ${packageManager[1]}, received ${actualPnpm}`,
    );
  }
  const output = spawnPnpm(root, [
    "why",
    "--json",
    "cross-fetch",
    "node-fetch",
    "whatwg-url",
    "tr46",
    "punycode",
  ]);
  let document;
  try {
    document = JSON.parse(output);
  } catch (error) {
    throw new Error(
      `pnpm why did not return strict JSON: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }
  return { pnpm: actualPnpm, records: analyzeWhyGraph(document) };
}

export function reconcileLockAndWhy(lockPackages, whyRecords) {
  const lock = new Set(
    lockPackages.map((entry) => `${entry.name}@${entry.version}`),
  );
  const why = new Set(
    whyRecords.map((entry) => `${entry.name}@${entry.version}`),
  );
  const unexplainedLock = [...lock].filter((entry) => !why.has(entry));
  const unexplainedWhy = [...why].filter((entry) => !lock.has(entry));
  if (unexplainedLock.length > 0 || unexplainedWhy.length > 0) {
    throw new Error(
      `pnpm lock/why graph mismatch; lock-only=[${unexplainedLock.join(
        ", ",
      )}] why-only=[${unexplainedWhy.join(", ")}]`,
    );
  }
  return {
    explainedPackages: [...lock].sort(),
    reversePaths: [...new Set(whyRecords.map((entry) => entry.path))].sort(),
  };
}

export function validateRuntime(root = ROOT) {
  const expected = read(root, ".node-version").trim();
  if (!/^\d+\.\d+\.\d+$/.test(expected)) {
    throw new Error(".node-version must contain one exact Node.js version");
  }
  if (process.versions.node !== expected) {
    throw new Error(
      `Node.js version mismatch: expected ${expected}, received ${process.versions.node}`,
    );
  }
  const globals = ["fetch", "Headers", "Request", "Response"];
  for (const name of globals) {
    if (typeof globalThis[name] !== "function") {
      throw new Error(`Node.js ${expected} does not provide native ${name}`);
    }
  }
  if (Object.prototype.hasOwnProperty.call(globalThis.fetch, "polyfill")) {
    throw new Error("Global fetch carries a forbidden polyfill marker");
  }
  return { expected, actual: process.versions.node, globals };
}

function executionConfigEntries(root) {
  const manifest = JSON.parse(read(root, "package.json"));
  const entries = Object.entries(manifest.scripts ?? {}).map(
    ([name, value]) => ({
      file: `package.json#scripts.${name}`,
      source: String(value),
    }),
  );
  entries.push(
    {
      file: "process.env.NODE_OPTIONS",
      source: process.env.NODE_OPTIONS ?? "",
    },
    {
      file: "process.env.NODE_NO_WARNINGS",
      source:
        process.env.NODE_NO_WARNINGS === undefined
          ? ""
          : `NODE_NO_WARNINGS=${process.env.NODE_NO_WARNINGS}`,
    },
  );
  for (const relativeRoot of [".github/workflows", ".mise/tasks"]) {
    const absoluteRoot = path.join(root, relativeRoot);
    for (const entry of fs.readdirSync(absoluteRoot, { withFileTypes: true })) {
      if (!entry.isFile() || !/\.(?:ya?ml|toml)$/.test(entry.name)) continue;
      const relative = `${relativeRoot}/${entry.name}`;
      entries.push({ file: relative, source: read(root, relative) });
    }
  }
  for (const relative of [
    "mise.toml",
    ".codex/config.toml",
    ".codex/hooks.json",
  ]) {
    entries.push({ file: relative, source: read(root, relative) });
  }
  const scriptsRoot = path.join(root, "scripts");
  const visitScripts = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        visitScripts(absolute);
        continue;
      }
      if (
        !entry.isFile() ||
        !EXECUTABLE_SCRIPT_EXTENSIONS.has(
          path.extname(entry.name).toLowerCase(),
        )
      ) {
        continue;
      }
      const relative = path.relative(root, absolute).split(path.sep).join("/");
      // This checker necessarily names every forbidden token in its detector.
      // All other executable scripts remain in the suppression scan.
      if (relative === "scripts/tasks/dep0040-check.mjs") continue;
      entries.push({
        file: relative,
        source: fs.readFileSync(absolute, "utf8"),
      });
    }
  };
  visitScripts(scriptsRoot);
  return entries;
}

function staticallyComposedStrings(source, filePath) {
  if (!MODULE_EXTENSIONS.has(path.extname(filePath).toLowerCase())) return [];
  const sourceFile = ts.createSourceFile(
    filePath,
    source,
    ts.ScriptTarget.Latest,
    true,
    scriptKind(filePath),
  );
  if (sourceFile.parseDiagnostics.length > 0) {
    const diagnostic = sourceFile.parseDiagnostics[0];
    throw new Error(
      `Cannot inspect execution script ${filePath}: ${ts.flattenDiagnosticMessageText(
        diagnostic.messageText,
        " ",
      )}`,
    );
  }

  const bindings = new Map();
  const collectBindings = (node) => {
    if (
      ts.isVariableDeclaration(node) &&
      ts.isIdentifier(node.name) &&
      node.initializer
    ) {
      bindings.set(node.name.text, node.initializer);
    }
    ts.forEachChild(node, collectBindings);
  };
  collectBindings(sourceFile);

  function evaluateArray(node, resolving = new Set()) {
    if (ts.isParenthesizedExpression(node)) {
      return evaluateArray(node.expression, resolving);
    }
    if (ts.isIdentifier(node) && bindings.has(node.text)) {
      if (resolving.has(node.text)) return undefined;
      const next = new Set(resolving);
      next.add(node.text);
      return evaluateArray(bindings.get(node.text), next);
    }
    if (!ts.isArrayLiteralExpression(node)) return undefined;
    const values = node.elements.map((entry) => evaluate(entry, resolving));
    return values.every((entry) => entry !== undefined) ? values : undefined;
  }

  function evaluate(node, resolving = new Set()) {
    if (!node) return undefined;
    if (ts.isStringLiteralLike(node)) return node.text;
    if (ts.isParenthesizedExpression(node)) {
      return evaluate(node.expression, resolving);
    }
    if (ts.isIdentifier(node) && bindings.has(node.text)) {
      if (resolving.has(node.text)) return undefined;
      const next = new Set(resolving);
      next.add(node.text);
      return evaluate(bindings.get(node.text), next);
    }
    if (
      ts.isBinaryExpression(node) &&
      node.operatorToken.kind === ts.SyntaxKind.PlusToken
    ) {
      const left = evaluate(node.left, resolving);
      const right = evaluate(node.right, resolving);
      return left === undefined || right === undefined
        ? undefined
        : `${left}${right}`;
    }
    if (ts.isTemplateExpression(node)) {
      let value = node.head.text;
      for (const span of node.templateSpans) {
        const expression = evaluate(span.expression, resolving);
        if (expression === undefined) return undefined;
        value += expression + span.literal.text;
      }
      return value;
    }
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression)
    ) {
      const receiver = node.expression.expression;
      const method = node.expression.name.text;
      if (method === "join") {
        const separator =
          node.arguments.length === 0
            ? ","
            : evaluate(node.arguments[0], resolving);
        const values = evaluateArray(receiver, resolving);
        if (separator !== undefined && values !== undefined) {
          return values.join(separator);
        }
      }
      if (method === "concat") {
        const values = [receiver, ...node.arguments].map((entry) =>
          evaluate(entry, resolving),
        );
        if (values.every((entry) => entry !== undefined)) {
          return values.join("");
        }
      }
    }
    return undefined;
  }

  const values = new Set();
  const visit = (node) => {
    const value = evaluate(node);
    if (value !== undefined) values.add(value);
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return [...values];
}

export function findSuppressionViolations(entries) {
  const violations = [];
  const prohibited = [
    ["NODE_NO_WARNINGS", /\bNODE_NO_WARNINGS\b/i],
    ["--no-warnings", /--no-warnings(?![A-Za-z0-9_-])/i],
    ["--no-deprecation", /--no-deprecation(?![A-Za-z0-9_-])/i],
    [
      "--disable-warning=DEP0040",
      /--disable-warning(?:=|\s+)DEP0040(?![A-Za-z0-9_-])/i,
    ],
    [
      "Node stderr filtering",
      /(?:node|pnpm|vitest)[^\n]*(?:2>\s*(?:\/dev\/null|NUL|\$null)|2>&1\s*\|\s*(?:grep|rg|Select-String))/i,
    ],
    [
      "deprecation stderr filtering",
      /(?:(?:grep|rg)[^\n]*(?:-v|--invert-match)|Select-String[^\n]*-NotMatch)[^\n]*(?:DEP0040|deprecat|warning)/i,
    ],
  ];
  for (const entry of entries) {
    const sources = [
      entry.source,
      ...staticallyComposedStrings(entry.source, entry.file),
    ];
    const matched = new Set();
    for (const [label, pattern] of prohibited) {
      if (
        sources.some((source) => pattern.test(source)) &&
        !matched.has(label)
      ) {
        violations.push(`${entry.file}: ${label}`);
        matched.add(label);
      }
    }
  }
  return violations;
}

export function validateNoSuppression(root = ROOT) {
  const entries = executionConfigEntries(root);
  const violations = findSuppressionViolations(entries);
  if (violations.length > 0) {
    throw new Error(
      `Deprecation warning suppression is prohibited: ${violations.join("; ")}`,
    );
  }
  return { executionConfigs: entries.length, violations: [] };
}

export function runDep0040Check(root = ROOT) {
  const checks = [];
  const evaluate = (name, callback) => {
    try {
      const detail = callback();
      checks.push({ name, status: "pass", detail });
      return detail;
    } catch (error) {
      checks.push({
        name,
        status: "fail",
        error: error instanceof Error ? error.message : String(error),
      });
      return undefined;
    }
  };

  evaluate("runtime", () => validateRuntime(root));
  evaluate("manifest", () => validateManifest(root));
  evaluate("module-specifiers", () => validateActiveModuleSpecifiers(root));
  const lock = evaluate("pnpm-lock", () =>
    parsePnpmLock(read(root, "pnpm-lock.yaml")),
  );
  const why = evaluate("pnpm-why", () => readWhyGraph(root));
  evaluate("lock-why-reconciliation", () => {
    if (!lock || !why) {
      throw new Error(
        "Lock and why graph must both pass before reconciliation",
      );
    }
    return reconcileLockAndWhy(lock.packages, why.records);
  });
  evaluate("warning-suppression", () => validateNoSuppression(root));

  return {
    schemaVersion: 1,
    status: checks.every((check) => check.status === "pass") ? "pass" : "fail",
    checks,
  };
}

if (
  process.argv[1] &&
  pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url
) {
  if (process.argv.length !== 2) {
    console.log(
      JSON.stringify(
        {
          schemaVersion: 1,
          status: "fail",
          checks: [
            {
              name: "arguments",
              status: "fail",
              error: "Usage: dep0040-check.mjs",
            },
          ],
        },
        null,
        2,
      ),
    );
    process.exitCode = 1;
  } else {
    const report = runDep0040Check();
    console.log(JSON.stringify(report, null, 2));
    if (report.status !== "pass") process.exitCode = 1;
  }
}
