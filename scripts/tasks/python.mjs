#!/usr/bin/env node

import process from "node:process";
import {
  assertSimplePackageNames,
  fail,
  printPlan,
  run,
  usageBoolean,
  usageList,
  usageValue,
} from "./lib.mjs";

function validatedRequirementList(name) {
  const values = usageList(name);
  if (values.length === 0)
    throw new Error("At least one requirement is required");
  for (const value of values) {
    if (/\s/.test(value) || value.startsWith("-")) {
      throw new Error(`Invalid Python requirement: ${value}`);
    }
  }
  return values;
}

function mutate(title, args) {
  if (!usageBoolean("apply")) {
    printPlan(title, "uv", args);
    return;
  }
  run("uv", args);
}

try {
  switch (process.argv[2]) {
    case "lock":
      if (usageBoolean("apply")) run("uv", ["lock"]);
      else run("uv", ["lock", "--check", "--offline"]);
      break;
    case "add-dev":
      mutate("add Python development dependencies", [
        "add",
        "--dev",
        ...validatedRequirementList("requirements"),
      ]);
      break;
    case "remove-dev":
      mutate("remove Python development dependencies", [
        "remove",
        "--dev",
        ...assertSimplePackageNames(usageList("packages"), "package"),
      ]);
      break;
    case "update": {
      const packages = assertSimplePackageNames(
        usageList("packages"),
        "package",
      );
      const args = [
        "lock",
        ...packages.flatMap((name) => ["--upgrade-package", name]),
      ];
      mutate("update locked Python dependencies", args);
      break;
    }
    case "with": {
      const requirement = usageValue("requirement");
      const command = usageList("command");
      if (
        !requirement ||
        /\s/.test(requirement) ||
        requirement.startsWith("-")
      ) {
        throw new Error("A valid temporary Python requirement is required");
      }
      if (command.length === 0) throw new Error("A command is required");
      run("uv", ["run", "--with", requirement, "--", ...command]);
      break;
    }
    case "tool": {
      const command = usageList("command");
      if (command.length === 0) throw new Error("A tool command is required");
      run("uv", ["tool", "run", ...command]);
      break;
    }
    case "run": {
      const command = usageList("command");
      if (command.length === 0) throw new Error("A Python command is required");
      run("uv", ["run", "--locked", "--", ...command]);
      break;
    }
    default:
      throw new Error(`Unknown Python task command: ${process.argv[2] ?? ""}`);
  }
} catch (error) {
  fail(error);
}
