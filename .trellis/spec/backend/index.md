# Backend Development Guidelines

This layer records executable backend contracts for the Rust/Tauri host. It
supplements the versioned FyAgent product documents. For application-version,
release-metadata, and MSI-directory-policy work, `docs/fyagent/dev/v1-0.2.1/`
is the raw reference package; preserve it byte-for-byte and record checkout
adaptations in the code-specs below. For the active v1.0.2 product scope,
`docs/fyagent/dev/v1-0.2/` remains authoritative where it changes a
non-superseded contract; `docs/fyagent/dev/v1-0.0/` is the historical
initial-design record. When a code contract changes, update its relevant
code-spec and enforcing test; update a product document only when that document
is owned by the change.

## Guidelines

| Guide                                                                          | Use it for                                                                                                                     |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| [Codex Desktop Installer](./codex-desktop-installer.md)                        | The fixed-source installer service, IPC DTOs, job events, and platform boundaries.                                             |
| [FyAgent 0.2.1 Version and Installer](./fyagent-version-contract.md)           | Cargo version single source, version commands, frozen release values, native MSI directory policy, and release gates.          |
| [GitHub Release Workflow](./github-release-workflow.md)                        | Manually dispatched unsigned macOS artifacts and signed tag release boundaries.                                                |
| [WSL macOS Universal DMG](./wsl-macos-cross-build.md)                          | Pinned WSL2 cross-toolchain, Universal-only experimental DMG, and native validation boundaries.                                |
| [Development Environment](./development-environment.md)                        | mise-first local tool versions, compatibility declarations, platform boundaries, and WSL PATH isolation.                       |
| [Application Brand Assets](./application-brand-assets.md)                      | Cross-platform app icons, About reuse, macOS tray templates, and validation.                                                   |
| [Application Identity](./application-identity.md)                              | Cross-layer FyAgent identity, clean-break behavior, and provenance exceptions.                                                 |
| [Deep-Link Import Security](./deeplink-import-security.md)                     | Untrusted `fyagent://` request validation, explicit provider activation approval, and credential-safe confirmation behavior.   |
| [FyAgent v1-0.1 Configuration Domains](./fyagent-v1-0-1-config-domains.md)     | Codex capability/restart contracts and WorkBuddy's isolated secure configuration domain; versioning lives in its own contract. |
| [Windows Formal Release and Runtime Activation](./windows-release-boundary.md) | Explicit elevated-release manifest selection, protected activation forwarding, and pre-CLI privilege gates.                    |
