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

## Pre-Development Checklist

Before changing Rust/Tauri host code:

1. Locate the owning contract below; do not add a second rule where an existing
   installer, version, security, release, or platform contract already owns it.
2. For a Tauri command, serialized DTO, event, or persisted-data change, also
   read the [Frontend Development Guidelines](../frontend/index.md) and its
   [Type Safety](../frontend/type-safety.md) boundary before changing either
   side.
3. For user files, credentials, deep links, process control, installers, or
   release artifacts, identify the validation/error case and the test that will
   prove it before editing implementation code.
4. Run local commands through the shared
   [Development Environment Contract](./development-environment.md); do not
   substitute a machine-global Node, Rust, or pnpm toolchain.
5. Keep versioned product documents in their documented ownership boundary;
   record checkout-specific implementation contracts here rather than
   mechanically rewriting historical/reference packages.

## Guidelines

| Guide                                                                          | Use it for                                                                                                                          |
| ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| [Codex Desktop Installer](./codex-desktop-installer.md)                        | The fixed-source installer service, IPC DTOs, job events, and platform boundaries.                                                  |
| [FyAgent 0.3.0 Version and Installer](./fyagent-version-contract.md)           | Cargo version single source, version commands, frozen release values, native MSI directory policy, and release gates.               |
| [GitHub CI Workflow](./github-ci-workflow.md)                                  | Automatic PR/main/merge-group CI, stable Required aggregation, pinned toolchains/Actions, and trusted-base Labeler.                 |
| [GitHub Release Workflow](./github-release-workflow.md)                        | Trusted-main same-SHA unsigned preflight, formal CI eligibility, exact evidence/attestations, and verified draft-to-stable publish. |
| [Development Environment](./development-environment.md)                        | mise-first local tool versions, compatibility declarations, platform boundaries, and WSL PATH isolation.                            |
| [Repository Task Runner](./task-runner-contract.md)                            | Canonical mise task metadata, parameter transport, DAG side effects, maintenance safety, and generated documentation.               |
| [Application Brand Assets](./application-brand-assets.md)                      | Cross-platform app icons, About reuse, macOS tray templates, and validation.                                                        |
| [Application Identity](./application-identity.md)                              | Cross-layer FyAgent identity, clean-break behavior, and provenance exceptions.                                                      |
| [CC Switch Upstream Synchronization](./upstream-sync.md)                       | Immutable upstream tag verification, two-parent merge ancestry, conflict precedence, and provenance boundaries.                     |
| [Deep-Link Import Security](./deeplink-import-security.md)                     | Untrusted `fyagent://` request validation, explicit provider activation approval, and credential-safe confirmation behavior.        |
| [FyAgent v1-0.1 Configuration Domains](./fyagent-v1-0-1-config-domains.md)     | Codex capability/restart contracts and WorkBuddy's isolated secure configuration domain; versioning lives in its own contract.      |
| [Windows Formal Release and Runtime Activation](./windows-release-boundary.md) | Explicit elevated-release manifest selection, protected activation forwarding, and pre-CLI privilege gates.                         |

## Quality Check

Run the **Tests Required** section of every contract affected by the change.
For ordinary Rust/Tauri changes, the baseline local checks are:

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

Add the applicable renderer, version, Windows, macOS, or release
contract checks rather than reporting an unrelated local command as platform or
release evidence. A code-spec update that changes a contract must name its
enforcing test; successful local static checks never prove a signed, notarized,
or remotely published artifact.
