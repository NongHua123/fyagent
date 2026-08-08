# Development Hooks Contract

## Scope

This contract owns repository-provided Codex hooks and the project entry points to Trellis Python scripts. It does not require every contributor to use Codex, and it does not permit hooks to alter the environment during a prompt.

## Execution chain

```text
Codex/Trellis entry
→ mise run <task>
→ mise-selected uv
→ uv-managed Python 3.14.7
→ project .venv and committed uv.lock
→ repository Python script
```

No current project workflow documents direct `python3`, `python`, `py`, or absolute system Python calls.

## Trellis commands

The canonical project interfaces are:

```text
trellis:init-developer
trellis:get-developer
trellis:context
trellis:task
trellis:session:add
trellis:validate
```

mise validates top-level required arguments and delegates detailed subcommand semantics to the existing Python `argparse` implementation. The Python scripts remain independently testable and do not call mise.

## Codex hooks

Hook tasks execute with:

```text
uv run --locked --no-sync --offline ...
```

They may not download uv/Python/packages, synchronize `.venv`, modify locks, grant trust, or access the network. `bootstrap` and `codex:hooks:check` prepare/verify the environment before normal use.

When the environment is absent or incomplete, the wrapper emits valid hook JSON with `continue: true` and a visible `systemMessage` instructing the developer to run `mise run bootstrap`. It exits successfully and does not block a prompt or subagent. A damaged script, invalid JSON, or unexpected internal exception is a real failure and is not silently converted to success.

## Privacy and side effects

Hooks inject only repository workflow context required by their documented purpose. They do not read product user credentials, external CLI configuration, signing secrets, or final-user data directories.

## Tests Required

- parse `.codex/hooks.json` and resolve every referenced task;
- execute each hook with simulated valid input and validate output schema;
- execute with missing `.venv` and verify visible non-blocking degradation;
- prove no network or environment synchronization in hook mode;
- verify current workflow/skills use `mise run trellis:*`;
- exclude generic `trellis-meta/references/**` from project command-rewrite rules.
