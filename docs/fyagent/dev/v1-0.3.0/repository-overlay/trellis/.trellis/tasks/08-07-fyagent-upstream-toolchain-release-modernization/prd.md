# FyAgent Modernization Parent PRD

## Objective

Deliver the confirmed Decisions 1–104 as six ordered implementation workstreams while preserving evidence, Git provenance, FyAgent identity, product behavior, and formal release integrity.

## Scope

1. complete explicit merge of CC Switch v3.19.2;
2. remove local Linux/WSL-to-Windows/macOS cross builds;
3. redesign mise/uv/Python and repository task API;
4. modernize Required CI and formal Release;
5. remove the DEP0040 dependency root cause;
6. migrate current documentation and Trellis contracts and supersede old tasks.

## Non-goals

- no product implementation in the planning package;
- no Git merge, commit, tag, push, GitHub configuration, test execution, or Release;
- no rewrite of historical design evidence;
- no conversion of optional product-runtime mise compatibility into a hard dependency.

## Cross-workstream invariants

- Upstream merge is an isolated explicit merge commit and happens first.
- FyAgent product identity, license boundary, data paths, features, and `0.2.x` version line remain.
- Formal release assets originate only from GitHub Actions.
- Local project operations use canonical `mise run` tasks.
- Evidence is labeled observed/decision/proposed/pending verification.
- Any NO-GO condition stops dependent work.

## Acceptance

- all six child tasks complete in dependency order;
- current specs and docs match implemented task/workflow behavior;
- Required CI passes on the protected branch;
- Release preflight proves the full ten-asset contract;
- risk register has no unresolved NO-GO item;
- source, decision, and file traceability are complete.
