#!/usr/bin/env node

import process from "node:process";
import { fail, run, usageList } from "./lib.mjs";

try {
  if (process.argv[2] !== "test") {
    throw new Error(`Unknown Rust task command: ${process.argv[2] ?? ""}`);
  }
  const filters = usageList("filters");
  if (
    filters.some(
      (filter) => filter.startsWith("-") || filter.includes("--target"),
    )
  ) {
    throw new Error(
      "rust:test accepts test-name filters only; Cargo options and targets are forbidden",
    );
  }
  if (filters.length > 1) {
    throw new Error("rust:test accepts at most one test-name filter");
  }
  run("cargo", [
    "test",
    "--workspace",
    "--locked",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    ...(filters.length === 1 ? ["--", filters[0]] : []),
  ]);
} catch (error) {
  fail(error);
}
