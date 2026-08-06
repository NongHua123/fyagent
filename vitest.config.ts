import path from "node:path";
import { configDefaults, defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setupGlobals.ts", "./tests/setupTests.ts"],
    globals: true,
    // The version contract uses Node's native test runner. Keep those tests
    // (and the immutable reference package) out of Vitest discovery so the
    // two runners can coexist in the project-level test command.
    exclude: [
      ...configDefaults.exclude,
      "**/*.test.mjs",
      "docs/fyagent/dev/v1-0.2.1/**",
    ],
    coverage: {
      reporter: ["text", "lcov"],
    },
  },
});
