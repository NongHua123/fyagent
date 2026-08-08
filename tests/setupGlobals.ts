import fs from "node:fs";
import path from "node:path";

const expectedNodeVersion = fs
  .readFileSync(path.resolve(process.cwd(), ".node-version"), "utf8")
  .trim();

if (
  !/^\d+\.\d+\.\d+$/.test(expectedNodeVersion) ||
  process.release.name !== "node" ||
  process.versions.node !== expectedNodeVersion
) {
  throw new Error(
    `FyAgent tests require the exact Node.js runtime from .node-version (${expectedNodeVersion}); received ${process.release.name} ${process.versions.node}.`,
  );
}

const requiredNativeWebApis = [
  "fetch",
  "Headers",
  "Request",
  "Response",
] as const;

for (const name of requiredNativeWebApis) {
  if (typeof globalThis[name] !== "function") {
    throw new Error(
      `FyAgent tests require native ${name} from the repository-pinned Node.js runtime; run mise run env:check and mise run bootstrap.`,
    );
  }
}

if (Object.prototype.hasOwnProperty.call(globalThis.fetch, "polyfill")) {
  throw new Error(
    "FyAgent tests refuse a Fetch implementation carrying a polyfill marker; use the repository-pinned Node.js runtime.",
  );
}

// Polyfill ResizeObserver for jsdom/happy-dom
if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof globalThis.ResizeObserver;
}

const storage = new Map<string, string>();

if (
  typeof globalThis.localStorage === "undefined" ||
  typeof globalThis.localStorage?.getItem !== "function"
) {
  Object.defineProperty(globalThis, "localStorage", {
    value: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => {
        storage.set(key, String(value));
      },
      removeItem: (key: string) => {
        storage.delete(key);
      },
      clear: () => {
        storage.clear();
      },
      key: (index: number) => Array.from(storage.keys())[index] ?? null,
      get length() {
        return storage.size;
      },
    },
    configurable: true,
  });
}

if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
}

if (!Element.prototype.setPointerCapture) {
  Element.prototype.setPointerCapture = () => {};
}

if (!Element.prototype.releasePointerCapture) {
  Element.prototype.releasePointerCapture = () => {};
}
