# Code, Configuration, and Workflow Target Snippets

> [Proposed / 拟实施] These snippets are design aids, not files claimed to be implemented. Reconcile them with the real tree after the upstream merge and use reviewed current Action SHAs/container digests.

## `mise.toml` ownership skeleton

```toml
min_version = "<derived-from-used-features>"

[settings]
lockfile = true
idiomatic_version_file_enable_tools = ["node", "pnpm", "rust"]

[tools]
uv = "latest"

[task_config]
includes = [".mise/tasks"]
```

Do not declare Python, Node, pnpm, or Rust versions again in this file. Do not disable auto-install globally.

## Python project

`.python-version`:

```text
3.14.7
```

`pyproject.toml` skeleton:

```toml
[project]
name = "fyagent-development-environment"
version = "0.0.0"
requires-python = ">=3.14,<3.15"
dependencies = []

[dependency-groups]
dev = []

[tool.uv]
package = false
python-preference = "only-managed"
python-downloads = "automatic"
required-version = ">=<minimum-capability-version>"
```

The actual approved uv version is resolved by `mise.lock`; `uv.lock` owns Python package resolution.

## Codex hooks target

`.codex/hooks.json` should reference canonical tasks, for example:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      { "command": "mise run --silent codex:hook:workflow-state" }
    ],
    "SubagentStart": [
      { "command": "mise run --silent codex:hook:subagent-context" }
    ]
  }
}
```

The task implementation uses `uv run --locked --no-sync --offline`. Confirm the exact hook schema against the installed Codex version before applying.

## Required CI shape

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  merge_group:
    types: [checks_requested]
  workflow_dispatch:

permissions:
  contents: read
```

Use full immutable Action SHAs, explicit runners, and a final `CI / Required` job that inspects every required dependency result. Node/Rust/pnpm versions are read from standard files. Do not copy placeholder SHAs from this document.

## Linux Release job shape

```yaml
runs-on: ubuntu-24.04 # or the reviewed explicit ARM64 counterpart
container:
  image: ubuntu:22.04@sha256:<reviewed-digest>
```

The host and container architectures match. No QEMU or cross-architecture target is used.

## Native Fetch remediation

```diff
- import "cross-fetch/polyfill";
```

Remove the direct `cross-fetch` dependency, regenerate `pnpm-lock.yaml` with pnpm 10.12.3, add a real native Fetch/MSW test, and run the focused pending-deprecation probe. Do not add a replacement polyfill or warning suppression.
