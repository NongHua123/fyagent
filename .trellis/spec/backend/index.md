# Backend Development Guidelines

This layer records executable backend contracts for the Rust/Tauri host. It
supplements the feature-specific V1 documents under docs/fyagent/dev/v1/;
when a contract changes, update both the relevant feature document and the
test that enforces it.

## Guidelines

| Guide                                                   | Use it for                                                                         |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [Codex Desktop Installer](./codex-desktop-installer.md) | The fixed-source installer service, IPC DTOs, job events, and platform boundaries. |
