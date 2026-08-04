# Backend Development Guidelines

This layer records executable backend contracts for the Rust/Tauri host. It
supplements the feature-specific V1 documents under docs/fyagent/dev/v1/;
when a contract changes, update both the relevant feature document and the
test that enforces it.

## Guidelines

| Guide                                                                      | Use it for                                                                                                                   |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| [Codex Desktop Installer](./codex-desktop-installer.md)                    | The fixed-source installer service, IPC DTOs, job events, and platform boundaries.                                           |
| [GitHub Release Workflow](./github-release-workflow.md)                    | Manually dispatched unsigned macOS artifacts and signed tag release boundaries.                                              |
| [WSL macOS Universal DMG](./wsl-macos-cross-build.md)                      | Pinned WSL2 cross-toolchain, Universal-only experimental DMG, and native validation boundaries.                              |
| [WSL Development Environment](./wsl-development-environment.md)            | mise-first WSL tool versions, compatibility declarations, PATH isolation, and native dependency boundaries.                  |
| [Application Brand Assets](./application-brand-assets.md)                  | Cross-platform app icons, About reuse, macOS tray templates, and validation.                                                 |
| [Application Identity](./application-identity.md)                          | Cross-layer FyAgent identity, clean-break behavior, and provenance exceptions.                                               |
| [FyAgent v1-0.1 Configuration Domains](./fyagent-v1-0-1-config-domains.md) | Independent `0.1.0` version chain, Codex capability/restart contracts, and WorkBuddy's isolated secure configuration domain. |
