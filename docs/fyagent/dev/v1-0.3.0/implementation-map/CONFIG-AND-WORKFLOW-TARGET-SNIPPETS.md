# Code, Configuration, and Workflow Implementation Crosswalk

> [Implemented locally / 本地已实施] These excerpts summarize the current tree. The real files and executable contracts are authoritative; this crosswalk does not replace them or prove remote Actions runs.

## `mise.toml` ownership skeleton

```toml
min_version = "2026.8.0"

[settings]
lockfile = true
task.run_auto_install = false
idiomatic_version_file_enable_tools = ["node", "pnpm", "rust"]

[tools]
uv = "latest"

[tool_alias]
pnpm = "github:pnpm/pnpm"
uv = "github:astral-sh/uv"

[task_config]
includes = [
  ".mise/tasks/core.toml",
  ".mise/tasks/frontend.toml",
  ".mise/tasks/rust.toml",
  ".mise/tasks/python.toml",
  ".mise/tasks/trellis.toml",
  ".mise/tasks/upstream.toml",
  ".mise/tasks/contracts.toml",
  ".mise/tasks/release.toml",
  ".mise/tasks/hooks.toml",
]
```

Do not declare Python, Node, pnpm, or Rust versions again in this file. D115 disables task-triggered implicit installation, not mise's global tool capability: `mise run bootstrap` performs the explicit locked install before ordinary tasks run.

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
```

The actual approved uv version is resolved by `mise.lock`; `uv.lock` owns Python package resolution.

## Codex hooks target

`.codex/hooks.json` references canonical tasks:

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

The implemented task uses `uv run --locked --no-sync --offline`; protocol and degradation tests are part of the local contract.

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

The current workflow uses reviewed full immutable Action SHAs, explicit runners, and a final `CI / Required` job that inspects the exact six dependency results. Node/Rust/pnpm versions are read from standard files. The project owner accepted D113/D114 on 2026-08-08. Real PR/main/manual evidence remains pending; D114 records a live `merge_group` run as N/A under the current personal-repository/no-protection governance, not as success. Its accepted substitute is the YAML trigger, fail-closed contract/static tests, and those real PR/main/manual runs.

## Linux Release job shape

```yaml
runs-on: ubuntu-24.04 # x64; ubuntu-24.04-arm for ARM64
container:
  # x64 shown; ARM64 uses its reviewed architecture-specific manifest.
  image: ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e
```

The host and container architectures match. No QEMU or cross-architecture target is used.

## Native Fetch remediation

```diff
- import "cross-fetch/polyfill";
```

Commit `4e407df4` removed the direct `cross-fetch` dependency, regenerated `pnpm-lock.yaml` with pnpm 10.12.3, and added real native Fetch/MSW behavior plus focused pending-deprecation and suppression contracts. No replacement polyfill was added.
