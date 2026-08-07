#!/usr/bin/env node

import process from "node:process";
import { fail, run, usageList, usageValue } from "./lib.mjs";

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

try {
  const command = process.argv[2];
  if (command === "validate") {
    const task = usageValue("task");
    invoke(scripts.task, ["validate", ...(task ? [task] : [])]);
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
    invoke(scripts[command], usageList("args"));
  } else if (command === "task" || command === "session-add") {
    const args = usageList("args");
    if (args.length === 0)
      throw new Error("At least one forwarded argument is required");
    invoke(scripts[command], args);
  } else {
    throw new Error(`Unknown Trellis task command: ${command ?? ""}`);
  }
} catch (error) {
  fail(error);
}
