# Backend Development Guidelines

This layer contains current executable engineering contracts. Historical product documents under `docs/fyagent/dev/**` are preserved evidence and are not the current command, toolchain, CI, release, or upstream synchronization authority.

## Pre-development checklist

1. Identify the owning contract and its enforcing test before changing behavior.
2. Use `mise run` tasks from the Development Environment and Task Runner contracts.
3. Treat user files, credentials, deep links, processes, installers, and release assets as explicit validation/security boundaries.
4. For cross-layer DTO/event/persistence changes, also read the frontend type-safety and state contracts.
5. Update a lasting spec and its test in the same workstream.

## Contracts

| Guide | Owns |
|---|---|
| [Development Environment](./development-environment.md) | Node/pnpm/Rust/mise/uv/Python, locks, bootstrap and strict checks |
| [Task Runner](./task-runner-contract.md) | Canonical `mise run` API, parameters, side effects and task docs |
| [Upstream Synchronization](./upstream-sync.md) | origin/upstream identity, tag merge, conflict and provenance rules |
| [Development Hooks](./development-hooks.md) | Trellis wrappers, uv Python and Codex hook degradation |
| [GitHub CI and Release](./github-release-workflow.md) | Required CI, runners, permissions, assets and provenance |
| [Windows Native Release](./windows-release-boundary.md) | Native MSI runners, manifests, signing and installer security |
| [Codex Desktop Installer](./codex-desktop-installer.md) | Installer service and platform identity boundaries |
| [FyAgent Version Contract](./fyagent-version-contract.md) | Product version source and version mutation contract |
| [Application Brand Assets](./application-brand-assets.md) | Icon/tray/About generation and validation |
| [Application Identity](./application-identity.md) | FyAgent identity and provenance exceptions |
| [Deep-Link Import Security](./deeplink-import-security.md) | Untrusted deep-link validation and credential-safe approval |
| [Configuration Domains](./fyagent-v1-0-1-config-domains.md) | Domain isolation and restart/config contracts |

`wsl-macos-cross-build.md` is retired and removed from the active index. Git history retains the previous contract.

## Baseline quality gate

```text
mise run check
```

This is a current-host gate only. GitHub Required CI and the formal Release workflow own multi-platform and publication evidence.
