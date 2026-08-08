#!/usr/bin/env node

import process from "node:process";
import { capture, fail, run, usageList, usageValue } from "./lib.mjs";

const scripts = Object.freeze({
  "init-developer": ".trellis/scripts/init_developer.py",
  "get-developer": ".trellis/scripts/get_developer.py",
  context: ".trellis/scripts/get_context.py",
  task: ".trellis/scripts/task.py",
  "session-add": ".trellis/scripts/add_session.py",
});

function invoke(script, args = []) {
  run("uv", ["run", "--locked", "python", script, ...args]);
}

function activeTaskDirectories() {
  const output = capture("uv", [
    "run",
    "--locked",
    "python",
    scripts.task,
    "list",
    "--json",
  ]);
  const report = JSON.parse(output);
  if (!report || !Array.isArray(report.tasks)) {
    throw new Error("Trellis task list did not return a tasks array");
  }

  const directories = report.tasks.map((task) => task?.dir);
  if (
    directories.some(
      (directory) =>
        typeof directory !== "string" ||
        !/^\.trellis\/tasks\/[A-Za-z0-9][A-Za-z0-9._-]*$/u.test(directory),
    )
  ) {
    throw new Error("Trellis task list returned an invalid active task path");
  }
  if (new Set(directories).size !== directories.length) {
    throw new Error("Trellis task list returned duplicate active task paths");
  }
  return directories;
}

function forwardedArguments(name) {
  const usageArguments = usageList(name);
  const rawArguments = process.argv.slice(3);
  if (usageArguments.length > 0 && rawArguments.length > 0) {
    throw new Error("Trellis arguments were provided through two channels");
  }
  return usageArguments.length > 0 ? usageArguments : rawArguments;
}

try {
  const command = process.argv[2];
  if (command === "validate") {
    const task = usageValue("task");
    const directories = task ? [task] : activeTaskDirectories();
    for (const directory of directories) {
      invoke(scripts.task, ["validate", directory]);
    }
    if (!task) {
      console.log(`Validated ${directories.length} active Trellis task(s)`);
    }
  } else if (command === "init-developer") {
    const name = usageValue("name");
    if (!name || !/^[\w.-]+$/u.test(name)) {
      throw new Error(
        "Developer identity may contain only letters, digits, _, -, and .",
      );
    }
    invoke(scripts[command], [name]);
  } else if (command === "get-developer") {
    invoke(scripts[command]);
  } else if (command === "context") {
    invoke(scripts[command], forwardedArguments("args"));
  } else if (command === "task" || command === "session-add") {
    const args = forwardedArguments("args");
    if (args.length === 0)
      throw new Error("At least one forwarded argument is required");
    invoke(scripts[command], args);
  } else {
    throw new Error(`Unknown Trellis task command: ${command ?? ""}`);
  }
} catch (error) {
  fail(error);
}
