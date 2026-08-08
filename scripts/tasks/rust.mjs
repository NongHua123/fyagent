#!/usr/bin/env node

import process from "node:process";
import { executeCargoTask } from "./host-native.mjs";
import { fail, usageList } from "./lib.mjs";

try {
  const operation = process.argv[2];
  executeCargoTask({
    operation,
    filters: operation === "test" ? usageList("filters") : [],
    forwardedArguments: process.argv.slice(3),
  });
} catch (error) {
  fail(error);
}
