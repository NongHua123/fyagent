# Codex Development Hook Contract

## 1. Scope / Trigger

Read this contract before changing `.codex/hooks.json`, either registered Codex
Python hook, `.mise/tasks/hooks.toml`, `pyproject.toml`, `uv.lock`, or
`scripts/tasks/codex-hook-runner.mjs`.

The hooks expose Trellis workflow and curated task context to Codex. They are a
prompt-assistance boundary, not an environment bootstrapper: a prompt may
continue without injected context when the valid project environment has not
yet prepared `.venv`, but malformed protocol or damaged project state must not
be reported as a successful hook invocation.

## 2. Signatures

### Codex registration

```text
UserPromptSubmit:
  mise run --silent --skip-tools --deny-net codex:hook:workflow-state

SubagentStart (trellis-implement|trellis-check|trellis-research):
  mise run --silent --skip-tools --deny-net codex:hook:subagent-context
```

Both registrations keep the nested Codex hook schema and a 15-second timeout.
They must not call Python, uv, or a hook script directly.

The three task names are stable:

```text
codex:hook:workflow-state
codex:hook:subagent-context
codex:hooks:check
```

Every task is read-only metadata and `raw = true`. Raw mode is required because
stdin is one hook JSON object and stdout is one hook JSON object; line prefixes,
task labels, buffering wrappers, or status prose corrupt the protocol.

### Python execution

The Node runner is the only task entry point. A prepared environment always
uses this exact uv prefix:

```text
uv run --locked --no-sync --offline python -X utf8 <hook-script>
```

`--locked` rejects project/lock drift. `--no-sync` and `--offline` prohibit a
prompt from creating or repairing `.venv`, resolving packages, downloading a
Python build, or changing the lock. The runner also disables bytecode writes so
the hook cannot create `__pycache__` as a prompt side effect.

## 3. Project and Protocol Contracts

Before deciding whether `.venv` is ready, the runner uses a dependency-free,
minimal TOML scope parser and validates:

- `.python-version` contains exactly `3.14.7`;
- `pyproject.toml` declares `[project].requires-python = ">=3.14,<3.15"` and
  declares `[tool.uv].package = false`, `python-preference = "only-managed"`,
  and `python-downloads = "automatic"` exactly once in those tables;
- `uv.lock` declares top-level `version = 1`, `revision = 3`, and the
  uv-normalized `requires-python = "==3.14.*"` exactly once;
- the selected registered Python hook is a regular, non-empty file whose
  line-ending-normalized SHA-256 matches the reviewed runner allowlist, so LF
  and CRLF checkouts share one integrity identity.

For `UserPromptSubmit`, stdin must be an object whose event is exactly
`UserPromptSubmit`. For `SubagentStart`, the event must be exactly
`SubagentStart`, `agent_type` must be one of the three registered Trellis roles,
and `session_id` must be non-empty. A supplied `cwd` must be a string inside the
FyAgent repository.

A successful Python process must exit zero, write no stderr, and emit exactly
one JSON object. That object is either a non-blocking `{ "continue": true }`
no-context response or contains a matching `hookSpecificOutput.hookEventName`
and non-empty `additionalContext`. Empty, multiple, non-JSON, wrong-event, or
`continue: false` output is a protocol failure.

The runner sets `FYAGENT_CODEX_HOOK_STRICT=1` for registered Codex events. This
makes missing, timed-out, malformed, or non-object stdin and internal native
SubagentStart errors fail closed. The Python scripts retain their established
generic fallback behavior when another platform invokes them without that
Codex-only marker.

## 4. Readiness and Error Matrix

| Condition                                                                                                                                          | Required result                                                                                             |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `TRELLIS_HOOKS=0` or `TRELLIS_DISABLE_HOOKS=1`                                                                                                     | Exit zero with no output; this is the only runner-level silent result.                                      |
| Valid project and hook files, but `.venv` is missing or incomplete                                                                                 | Emit `continue: true` plus visible `mise run bootstrap` guidance; do not invoke uv and do not modify files. |
| `.python-version`, `pyproject.toml`, or `uv.lock` is missing, malformed, duplicates a required key, uses a wrong table, or has an unapproved value | Non-zero exit with no success JSON.                                                                         |
| Registered Python hook is missing, empty, or differs from its reviewed hash                                                                        | Non-zero exit before environment degradation.                                                               |
| Hook event, role, session, cwd, or stdin JSON is invalid                                                                                           | Non-zero exit before invoking uv.                                                                           |
| uv is missing, times out, is signalled, or exits non-zero                                                                                          | Non-zero exit; do not reinterpret this as an unprepared environment.                                        |
| Hook writes stderr or invalid stdout                                                                                                               | Non-zero exit; do not forward partial output.                                                               |
| Valid hook emits context or an explicit no-context response                                                                                        | Normalize and forward one JSON object unchanged in meaning.                                                 |

Project or lock damage is different from first-time environment readiness. The
former requires repair by a developer; the latter is an expected pre-bootstrap
state and must not block the user's prompt.

The `.venv` root must be a real repository-local directory rather than a
symlink or junction. Otherwise the hook could execute or mutate environment
state outside the tree covered by the side-effect snapshot. The registered
event `cwd` must also resolve within the repository. Reject symlink escapes; on
Windows a path from a different drive is outside even when `path.relative`
returns an absolute path instead of a `..` prefix.

## 5. Side-Effect Boundary

Hook execution must not run `mise trust`, `mise install`, `uv sync`, `uv lock`,
`uv add`, pip installation, system package installation, Git commands, network
requests, or release commands. It must not update hashes, mtimes, or membership
under `.venv`, `pyproject.toml`, `uv.lock`, or `.python-version`.

`codex:hooks:check` requires a prepared environment. It runs both registered
paths with contract fixtures, snapshots the Python project and `.venv` before
and after, and fails if content, hashes, mtimes, or tree membership change. It
does not bootstrap an environment on behalf of the caller.

## 6. Tests Required

- Parse `.codex/hooks.json` and assert the nested schema, exact mise commands,
  matcher, and 15-second timeouts.
- Resolve every task with `mise tasks info --json` and assert `raw = true` and
  the read-only effect metadata.
- Pipe a real JSON event through the exact mise command and parse stdout as one
  unprefixed JSON object.
- Test the exact uv argument vector and offline/no-sync environment with a
  controlled process adapter.
- Snapshot hashes and nanosecond mtimes around both prepared and unprepared
  paths.
- Cover explicit disablement, unprepared `.venv`, damaged hook hash, malformed
  lock, required TOML keys in the wrong table, duplicate keys, unapproved
  values, invalid JSON/event/role/session/cwd (including a Windows cross-drive
  path and a symlink escape), a symlinked `.venv`, child failure, unexpected
  stderr, and invalid stdout.
- Compile all Python hook files and run `codex:hooks:check` after bootstrap.

## 7. Wrong vs Correct

### Wrong

```text
python3 .codex/hooks/inject-workflow-state.py
uv run .codex/hooks/inject-workflow-state.py
uv run --locked .codex/hooks/inject-workflow-state.py
```

These bypass the managed task boundary or permit synchronization/network work
during a prompt.

### Correct

```text
mise run --silent --skip-tools --deny-net codex:hook:workflow-state
  -> raw read-only task
  -> uv run --locked --no-sync --offline python -X utf8 ...
```

The only recovery instruction for an unprepared valid environment is the
visible, user-invoked `mise run bootstrap` path.
