---
name: fyagent-trellis
description: "FyAgent project entrypoint for the repository's mise-backed Trellis lifecycle. Use when a contributor needs the canonical local setup, context, task, validation, or closeout route without duplicating the general Trellis skills."
---

# FyAgent Trellis Entry Point

Use the existing Trellis lifecycle skills for planning, implementation,
checking, spec updates, and closeout. This file only selects FyAgent's stable
command boundary; it does not copy or replace those skills.

## Environment boundary

Review and trust the repository configuration once, outside repository tasks,
then prepare the locked environment:

```bash
mise trust
mise run bootstrap
```

Repository Trellis operations run through the uv-managed Python environment:

```bash
mise run trellis:context
mise run trellis:context -- --mode phase
mise run trellis:task -- current --source
mise run trellis:validate -- .trellis/tasks/<task-dir>
```

Use `trellis-start` or `trellis-continue` to enter the lifecycle,
`trellis-before-dev` before implementation, `trellis-check` after changes,
`trellis-update-spec` for durable knowledge, and `trellis-finish-work` only
after work commits exist. Planning approval, implementation approval, quality
checks, work commits, task archive, and journal remain distinct gates.

## Boundaries

- Do not call Trellis Python scripts directly in routine project instructions.
- Do not make a repository task run `mise trust`, install system packages,
  change Git remotes, create a tag, push, or publish a Release.
- A child task may archive only after its own acceptance evidence is real. The
  modernization parent remains open until remote CI, formal Release, closeout
  evidence, and every child archive are complete.
