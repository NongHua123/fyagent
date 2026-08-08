---
name: fyagent-trellis
description: FyAgent-specific Trellis command and Python environment boundary. Use before operating Trellis tasks, context, sessions, or Codex workflow hooks in this repository.
---

# FyAgent Trellis Integration

## Invariant

```text
mise run trellis:* -> mise-selected uv -> uv-managed Python -> .trellis/scripts/*.py
```

Do not invoke project Trellis scripts through system `python3`, `python`, or `py`. Do not make a Trellis script call mise recursively.

## Initialization

```bash
mise trust
mise run bootstrap
mise run trellis:init-developer <name>
```

`mise trust` is a human decision. `bootstrap` prepares locked tools/dependencies; it does not modify trust or Git remotes.

## Canonical operations

```bash
mise run trellis:context
mise run trellis:context --mode packages
mise run trellis:context --mode phase --step <X.Y> --platform codex
mise run trellis:task -- create "<title>" --slug <slug>
mise run trellis:task -- start <task>
mise run trellis:task -- validate <task>
mise run trellis:session:add -- --title "..." --commit "..." --summary "..."
```

Detailed `task.py` subcommand semantics remain authoritative in the Python argparse implementation; the mise layer validates top-level inputs and provides the unified interpreter.

## Codex hooks

Repository hooks call `mise run codex:hook:*`. Internally they run uv with `--locked --no-sync --offline`. If the project environment is missing, return valid non-blocking hook JSON and visibly instruct the developer to run `mise run bootstrap`. Invalid hook code or invalid JSON is not silently ignored.

## Template boundary

Project operational skills and `.trellis/workflow.md` use the wrapper. Generic `trellis-meta/references/**` may retain native examples because they document the reusable Trellis architecture rather than FyAgent's daily command contract.
