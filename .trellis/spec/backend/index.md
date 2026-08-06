# Backend Development Guidelines

This layer records executable backend contracts for the Rust/Tauri host. It
supplements the versioned FyAgent product documents. For the active v1.0.2
scope, `docs/fyagent/dev/v1-0.2/` is authoritative where it changes a
contract; `docs/fyagent/dev/v1-0.0/` is the historical initial-design record.
When a contract changes, update both the relevant feature document and the
test that enforces it.

## Guidelines

| Guide                                                                      | Use it for                                                                                                                   |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| [Codex Desktop Installer](./codex-desktop-installer.md)                    | The fixed-source installer service, IPC DTOs, job events, and platform boundaries.                                           |
| [GitHub Release Workflow](./github-release-workflow.md)                    | Manually dispatched unsigned macOS artifacts and signed tag release boundaries.                                              |
| [WSL macOS Universal DMG](./wsl-macos-cross-build.md)                      | Pinned WSL2 cross-toolchain, Universal-only experimental DMG, and native validation boundaries.                              |
| [Development Environment](./development-environment.md)                    | mise-first local tool versions, compatibility declarations, platform boundaries, and WSL PATH isolation.                     |
| [Application Brand Assets](./application-brand-assets.md)                  | Cross-platform app icons, About reuse, macOS tray templates, and validation.                                                 |
| [Application Identity](./application-identity.md)                          | Cross-layer FyAgent identity, clean-break behavior, and provenance exceptions.                                               |
| [Deep-Link Import Security](./deeplink-import-security.md)                 | Untrusted `fyagent://` request validation, explicit provider activation approval, and credential-safe confirmation behavior. |
| [FyAgent v1-0.1 Configuration Domains](./fyagent-v1-0-1-config-domains.md) | Independent `0.1.0` version chain, Codex capability/restart contracts, and WorkBuddy's isolated secure configuration domain. |
| [Windows Formal Release and Runtime Activation](./windows-release-boundary.md) | Explicit elevated-release manifest selection, protected activation forwarding, and pre-CLI privilege gates.              |
