#!/usr/bin/env node

import process from "node:process";
import { capture, fail, run, usageBoolean, usageValue } from "./lib.mjs";

const ORIGIN = /^https:\/\/github\.com\/NongHua123\/fyagent(?:\.git)?$/i;
const UPSTREAM = /^https:\/\/github\.com\/farion1231\/cc-switch(?:\.git)?$/i;
const TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

function validateTag() {
  const tag = usageValue("tag");
  if (!tag || !TAG.test(tag))
    throw new Error("Upstream tag must be exact vX.Y.Z");
  return tag;
}

function activeMerge() {
  return (
    run("git", ["rev-parse", "-q", "--verify", "MERGE_HEAD"], {
      capture: true,
      allowFailure: true,
    }).status === 0
  );
}

function remoteSafety({ requireClean = false } = {}) {
  const origin = capture("git", ["remote", "get-url", "origin"]);
  const upstream = capture("git", ["remote", "get-url", "upstream"]);
  const upstreamPush = capture("git", [
    "remote",
    "get-url",
    "--push",
    "upstream",
  ]);
  if (!ORIGIN.test(origin))
    throw new Error(`Unexpected origin fetch URL: ${origin}`);
  if (!UPSTREAM.test(upstream))
    throw new Error(`Unexpected upstream fetch URL: ${upstream}`);
  if (upstreamPush !== "DISABLED") {
    throw new Error(`upstream push URL must be DISABLED, got: ${upstreamPush}`);
  }
  if (activeMerge()) throw new Error("An existing Git merge is active");
  if (requireClean && capture("git", ["status", "--porcelain"]) !== "") {
    throw new Error("A clean worktree and index are required");
  }
  return { origin, upstream, upstreamPush };
}

function requireLocalTag(tag) {
  const result = run("git", ["show-ref", "--verify", `refs/tags/${tag}`], {
    capture: true,
    allowFailure: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `Local tag refs/tags/${tag} is missing; run upstream:fetch first`,
    );
  }
}

function audit(tag) {
  requireLocalTag(tag);
  const type = capture("git", ["cat-file", "-t", `refs/tags/${tag}`]);
  const tagObject = capture("git", ["rev-parse", `refs/tags/${tag}`]);
  const peeledCommit = capture("git", [
    "rev-parse",
    `refs/tags/${tag}^{commit}`,
  ]);
  const head = capture("git", ["rev-parse", "HEAD"]);
  const mergeBase = capture("git", ["merge-base", "HEAD", peeledCommit]);
  const commits = capture("git", [
    "log",
    "--oneline",
    "--no-decorate",
    `${mergeBase}..${peeledCommit}`,
  ]);
  const diff = capture("git", [
    "diff",
    "--stat",
    `${mergeBase}..${peeledCommit}`,
  ]);
  console.log(
    JSON.stringify(
      { tag, type, tagObject, peeledCommit, head, mergeBase, commits, diff },
      null,
      2,
    ),
  );
}

try {
  switch (process.argv[2]) {
    case "check":
      console.log(
        JSON.stringify(remoteSafety({ requireClean: true }), null, 2),
      );
      break;
    case "fetch": {
      const tag = validateTag();
      remoteSafety();
      run("git", [
        "fetch",
        "--no-tags",
        "upstream",
        `refs/tags/${tag}:refs/tags/${tag}`,
      ]);
      audit(tag);
      break;
    }
    case "audit":
      remoteSafety();
      audit(validateTag());
      break;
    case "merge-prepare": {
      const tag = validateTag();
      remoteSafety({ requireClean: true });
      requireLocalTag(tag);
      audit(tag);
      if (!usageBoolean("apply")) {
        console.log(
          JSON.stringify(
            {
              status: "preview",
              command: [
                "git",
                "merge",
                "--no-ff",
                "--no-commit",
                `${tag}^{commit}`,
              ],
            },
            null,
            2,
          ),
        );
        break;
      }
      run("git", ["merge", "--no-ff", "--no-commit", `${tag}^{commit}`]);
      break;
    }
    case "merge-abort":
      if (!activeMerge()) throw new Error("No active Git merge can be aborted");
      run("git", ["merge", "--abort"]);
      break;
    default:
      throw new Error(
        `Unknown upstream task command: ${process.argv[2] ?? ""}`,
      );
  }
} catch (error) {
  fail(error);
}
