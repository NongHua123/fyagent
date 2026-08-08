# Windows Native Build and Formal Release Boundary

## Native-only policy

Windows x64 and Windows ARM64 formal MSI packages are built on native GitHub-hosted Windows runners. Linux/WSL cargo-xwin, Wine, WiX cross-host scripts, and local `build:cross-windows*` tasks are retired.

Windows contributors may run native development and local bundle tasks on their own host. Those outputs are development evidence, never formal Release assets.

## Manifest selection

```text
local dev/build/debug tasks  -> FYAGENT_WINDOWS_MANIFEST=dev
local check/clippy/test       -> FYAGENT_WINDOWS_MANIFEST=test
formal GitHub Release         -> FYAGENT_WINDOWS_MANIFEST=release
```

No standard local task may select `release`. The formal manifest is available only in the protected Release workflow. The build script fails when the required explicit choice is absent or invalid.

## Formal runners

```text
x64   -> windows-2022
ARM64 -> windows-11-arm
```

Node/pnpm/Rust versions are read from repository standard files. Release jobs use the same source SHA, Action full-SHA pins, and minimum permissions contract.

## MSI and security checks

A formal Windows job verifies:

- FyAgent product name, manufacturer, bundle/application identity, and version;
- expected architecture and installer filename;
- native installation-directory validation and protected-path policy;
- manifest/privilege selection;
- signature chain and timestamp where signing is enabled;
- no cross-build metadata or Linux path in the package;
- inclusion in the exact ten-asset release set and digest manifest.

## Runtime activation

Protected activation/forwarding and pre-CLI privilege gates remain governed by the existing security implementation. Removing local cross-builds does not weaken runtime identity or installer-directory validation.

## Tests Required

- Windows x64 native check/test/build;
- Windows ARM64 native check/test/build on the supported runner;
- manifest branch unit/contract tests;
- MSI metadata and directory-policy inspection;
- signed preflight and formal release signature verification;
- negative repository scan proving cargo-xwin/Wine/cross-build entry points are absent.
