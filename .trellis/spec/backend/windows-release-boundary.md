# Windows Formal Release and Runtime Activation Boundary

## 1. Scope / Trigger

This contract applies to the Windows formal release build, MSI packaging and
directory admission, early startup, single-instance activation forwarding, and
all renderer-facing commands that could probe or invoke user CLI tools. It is
mandatory because a manifest selection, MSI directory policy, elevation
decision, named-pipe endpoint, and pre-Tauri startup path form one cross-layer
privilege boundary.

It also applies when `scripts/windows-cross/build-windows-msi.sh` produces a
Linux-built Windows MSI candidate. Cross-compilation changes the host tools,
not the formal-release manifest or runtime privilege boundary.

It distinguishes a distributable formal release from development and test
artifacts. A release-profile test harness must remain test-manifest based;
being compiled with the `release` profile alone is not evidence that a binary
may require elevation.

## 2. Signatures

```text
FYAGENT_WINDOWS_MANIFEST = release | test | dev
FYAGENT_INSTALLER_ACTIONS_DLL = target-specific installer-actions DLL
TAURI_FYAGENT_INSTALLER_ACTIONS_DLL = WiX/Tauri-visible form of that DLL path
```

```text
mise run build:cross-windows:x64
mise run build:cross-windows:arm64
mise run build:cross-windows
./scripts/windows-cross/build-windows-msi.sh [--arch all|x64|arm64]

dist-bundle/windows/<version>/<arch>/...
```

```text
cargo:rustc-link-arg-tests=/MANIFEST:EMBED
cargo:rustc-link-arg-tests=/MANIFESTINPUT:<fyagent-test.manifest>
```

```rust
pub(crate) const fn formal_windows_build() -> bool;
pub fn early_windows_startup_gate() -> WindowsStartupDisposition;
pub fn runtime_privilege_status() -> RuntimePrivilegeStatus;
pub(crate) fn install_activation_handler<F>(handler: F)
    -> Result<(), WindowsStartupErrorCode>;

fn elevated_windows_cli_boundary_active_for(formal_windows_build: bool) -> bool;
```

```rust
pub unsafe extern "system" fn ValidateFyAgentInstallDirUi(
    install: MSIHANDLE,
) -> u32;
pub unsafe extern "system" fn ValidateFyAgentInstallDirExecute(
    install: MSIHANDLE,
) -> u32;
```

`early_windows_startup_gate` returns exactly one of:

```rust
WindowsStartupDisposition::Continue
WindowsStartupDisposition::ForwardedToExistingInstance
WindowsStartupDisposition::Blocked(WindowsStartupErrorCode)
```

The fixed descriptor includes a per-instance pipe nonce and activation
capability. The protocol has bounded `ActivationFrame`, `HandshakeFrame`, and
`ActivationAuthFrame` records; it does not accept a caller-provided pipe name
or an unbounded argv payload.

## 3. Contracts

- `build.rs` selects the embedded Windows manifest from
  `FYAGENT_WINDOWS_MANIFEST`. `release` is the only choice that enables
  `fyagent_windows_release`; `test` and `dev` use the ordinary-user manifest.
  An unset value in a release profile fails the build rather than guessing.
- The Linux-to-Windows MSI entrypoint exports
  `FYAGENT_WINDOWS_MANIFEST=release` inside each architecture's actual Tauri
  build subshell. Preflight success alone is not sufficient, and the variable
  must not be left unset or changed to `test` for a distributable candidate.
- The default local candidate publication root is
  `dist-bundle/windows/`. A successful invocation publishes the selected
  version under `dist-bundle/windows/<version>/`; `--output-dir` is the
  explicit local override. Only `build:cross-windows` builds and atomically
  publishes both architectures as one version tree.
- Application version resolution and the MSI helper's workspace/package
  relationship are defined by
  [FyAgent 0.2.1 Version and Installer Contract](./fyagent-version-contract.md).
  The cross-build script obtains the candidate version through version:get; it
  must not recover it from a tag, package.json, or tauri.conf.json.
- installer-actions is an independent Windows cdylib using the locked
  windows-sys dependency family. Build it separately for each target before
  Tauri bundling, pass the same verified file through both helper environment
  variables, and verify PE Machine, MSI summary architecture, and embedded
  Binary-stream bytes. Do not make the main application crate a cdylib or add
  Tauri to the helper.
- WiX calls the two Type 1 actions above. They share one native, fail-closed
  policy for normalized fixed local paths, reparse traversal, system/user/temp/
  ProgramData exclusions, trusted ancestors, owner/DACL/effective rights, and
  non-empty directory/product-marker admission. The policy must not depend on
  WMI, PowerShell, VBScript, JScript, or a target-machine script, and must not
  create files or modify an ACL while validating.
- A UI policy denial populates the stable FYAGENT_INSTALLDIR result properties
  and returns success so the user can choose another directory. Execute
  revalidates the same path and a following Type 19 action blocks file writes
  when invalid. Do not collapse this into a UI-only check or surface expected
  denial as Error 1720.
- Repair/upgrade clears public directory inputs, restores only the HKLM
  InstallDir anchor, and fails closed before file writes if that trusted anchor
  is unavailable. Directory validation may be skipped only for a strict pure
  uninstall whose exact INSTALLDIR component closure is validated from the
  rendered MSI tables. At this baseline the closure is
  CMP_UninstallShortcut, InstallDirectoryAcl, Path, and RegistryEntries.
- `mise run` does not auto-install missing tools before this workflow starts;
  the cross-build script's preflight requires the Windows toolchain to be
  prepared explicitly rather than treating task execution as provisioning.
- `fyagent-test.manifest` linker arguments use only
  `cargo:rustc-link-arg-tests`. Do not use the all-target
  `cargo:rustc-link-arg` form and do not try to cancel it for application
  binaries with `/MANIFEST:NO`; either pattern leaks test-manifest linker state
  into the formal binary and can disable or conflict with the resource emitted
  by `tauri-build`.
- The release manifest is `requireAdministrator`; the test manifest is
  `asInvoker`. The signed release workflow explicitly selects `release` and
  verifies the embedded application manifest in the target release executable
  before signing and again after MSI bundling. The post-bundle check deliberately
  rereads that executable rather than extracting or installing the MSI payload;
  it is not proof that the final signed MSI contains the expected manifest.
- `main` calls `early_windows_startup_gate` before creating the Tauri runtime.
  A formal release continues only when privilege status is available, elevated,
  locally administrative, and proven to match the interactive user. Any
  unavailable or mismatched proof blocks startup with a stable safe code.
- The runtime state and lease live under the WiX-owned protected ProgramData
  root. Static root/state/lease objects must be opened without following
  reparse points and must verify their expected canonical object type, owner,
  and DACL. The process may enable `SeRestorePrivilege` only for the narrow
  protected-object operation and must restore the previous privilege state by
  RAII.
- The state file uses deterministic instance lookup only; its live pipe name
  is a fresh high-entropy nonce and its activation capability is a separate
  secret. Do not replace either with a static pipe, global named mutex, PID
  lookup, image-name lookup, or path guess.
- A forwarding client opens the pipe with `CreateFileW` and
  `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION | SECURITY_EFFECTIVE_ONLY`.
  It sends a bounded challenge, verifies the server HMAC proof using the
  protected descriptor capability, then sends the per-request HMAC and argv.
  It must not send argv before the proof succeeds and must not use
  `CallNamedPipe`.
- The server authenticates the client identity and validates the HMAC-bound
  frame before handing decoded arguments to the activation handler. Corrupt,
  expired, unauthenticated, or stale-descriptor routes fail closed.
- In a formal Windows release,
  `get_tool_versions`, `run_tool_lifecycle_action`, and
  `probe_tool_installations` must stop at the elevated-CLI boundary before any
  user CLI search, probe, shell construction, or lifecycle command runs.
  Development/test builds preserve their existing ordinary-user behavior.

## 4. Validation & Error Matrix

| Condition                                                                                                                           | Required result                                                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `FYAGENT_WINDOWS_MANIFEST` is invalid                                                                                               | Build fails with the accepted values.                                                                                           |
| Formal release uses release profile without an explicit manifest selection                                                          | Build fails; it never silently chooses elevated or test behavior.                                                               |
| Linux-to-Windows MSI build omits `FYAGENT_WINDOWS_MANIFEST=release` from the architecture build subshell                            | `build.rs` fails before linking; do not weaken the fail-closed selection.                                                       |
| Test manifest uses all-target linker arguments or the application binary receives `/MANIFEST:NO`                                    | Reject the change; test linker arguments must be test-target-only and the formal binary must retain the `tauri-build` resource. |
| Formal release process is non-elevated, not a local administrator, lacks a privilege status, or does not match the interactive user | `early_windows_startup_gate` returns `Blocked` with the appropriate safe code before Tauri construction.                        |
| Development/test process is non-elevated                                                                                            | It may continue under the `asInvoker` test manifest.                                                                            |
| Runtime root/state/lease has unexpected owner, DACL, object type, path, or reparse point                                            | Treat it as unavailable/untrusted; do not read, delete, or use it for activation.                                               |
| Descriptor references a running owner but pipe open/handshake/proof fails                                                           | Do not send argv; return the activation-forward failure outcome.                                                                |
| Authentication HMAC, frame shape, client identity, or bounds check fails                                                            | Reject the connection without invoking the activation handler.                                                                  |
| Formal release reaches a user CLI command                                                                                           | Return the elevated-boundary message before probing or running the CLI.                                                         |
| Target executable manifest inspection or signing validation fails in release workflow                                               | Fail the workflow before publishing the artifact.                                                                               |
| Final signed MSI payload has not been extracted and inspected                                                                       | Keep final MSI-payload manifest acceptance pending; do not claim it was verified by the target-executable check.                |
| Helper architecture, MSI Binary bytes, custom-action table, or INSTALLDIR component closure drifts                                  | Fail cross-build/release structure verification before candidate publication or signing.                                        |
| UI validator rejects a path                                                                                                         | Show the recoverable policy dialog; do not raise Error 1720.                                                                    |
| Execute validator rejects a path or repair/upgrade lacks its HKLM anchor                                                            | Type 19 aborts before InstallValidate/InstallFiles.                                                                             |

## 5. Good / Base / Bad Cases

- Good: The signed release workflow and Linux-to-Windows MSI candidate build
  explicitly use `FYAGENT_WINDOWS_MANIFEST=release`; test targets alone receive
  the `asInvoker` linker input; startup proves the elevated process is the
  interactive local administrator; a second invocation proves the server
  endpoint before it forwards a bounded deep-link argv.
- Base: A normal developer build or test harness uses
  `FYAGENT_WINDOWS_MANIFEST=test` (or `dev`) and retains ordinary-user startup
  semantics with fake/platform-neutral tests.
- Bad: Leaving the cross-build manifest unset, sending the test manifest to all
  linker targets, cancelling application manifests with `/MANIFEST:NO`, tying
  `requireAdministrator` to every release-profile binary, accepting a state
  file by path alone, exposing a fixed well-known pipe, sending argv before
  endpoint proof, or allowing a formal elevated build to shell out to an
  arbitrary user CLI.

## 6. Tests Required

- `src-tauri/src/windows_runtime/mod.rs` unit tests must cover bounded frame
  encode/decode, tamper/control/trailing-data rejection, descriptor decision
  policy, privilege-gate behavior, SDDL policy predicates, and both server and
  request HMAC proofs.
- `tests/desktopSecurityBoundary.test.ts` must assert the narrow renderer
  capability/CSP boundary, absence of portable Windows distribution claims,
  visual-baseline LFS policy, and pre-probe formal-release CLI guard.
- `tests/releaseWorkflow.test.ts` currently asserts explicit release
  manifest selection and target-executable manifest/signing verification steps
  in the release workflow. Before treating this boundary as fully
  test-enforced, extend it to distinguish the post-bundle target-executable
  inspection from verification of the final MSI payload.
  It must also bind the Linux cross-build export to the actual architecture
  build environment, require `cargo:rustc-link-arg-tests` for the test manifest,
  and reject all-target manifest arguments and `/MANIFEST:NO` cancellation.
- It must also assert the native helper build precedes bundling, architecture
  bridge variables and MSI Binary checks are present, UI/Execute Type 1 plus
  Type 19 paths are scheduled, protected HKLM maintenance restoration is
  present, the rendered INSTALLDIR closure is exact, and the retired
  script/WMI validator is absent.
- The current Linux cross-build gate completes the requested MSI architecture
  build plus Linux-side structure and checksum checks. It does not itself run
  strict Windows-target Clippy with `FYAGENT_WINDOWS_MANIFEST=release` or link a
  release-profile Windows library test harness with
  `FYAGENT_WINDOWS_MANIFEST=test --no-run`. Those are required Windows
  release validations to add to a named gate or run explicitly before release;
  do not report them as passed merely because local workspace or cross-build
  checks pass. None of these static checks replace native Windows validation.
- Run the declared safe checks through mise, including Rust format/clippy and
  the targeted/unit frontend suite. Native UAC, registry, MSI, PackageManager,
  signing, and live named-pipe validation are separate Windows release
  validation and require explicit authorization; do not fabricate them from a
  non-Windows host.
- Native Windows acceptance additionally covers the default directory, a safe
  custom directory, an unsafe directory, /qn INSTALLDIR, repair, upgrade,
  uninstall, verbose MSI logging, and ICE validation for x64 and ARM64. Linux
  table checks are necessary structure evidence, not an equivalent lifecycle
  result. It must also extract and inspect the embedded executable manifest
  from the final signed MSI before treating that payload as release-validated.

## 7. Wrong vs Correct

### Wrong

```rust
let pipe = CallNamedPipeW(PCWSTR(STATIC_PIPE.as_ptr()), argv, ...);
```

This trusts a predictable endpoint and transmits user-controlled arguments
before proving who owns it.

### Correct

```rust
let pipe = CreateFileW(
    nonce_pipe_name,
    read_write,
    0,
    None,
    OPEN_EXISTING,
    FILE_ATTRIBUTE_NORMAL
        | SECURITY_SQOS_PRESENT
        | SECURITY_IDENTIFICATION
        | SECURITY_EFFECTIVE_ONLY,
    None,
)?;
verify_server_proof(capability, challenge, &proof)?;
write_activation_auth_and_frame(capability, challenge, argv)?;
```

The client obtains its non-static endpoint and secret only from a protected,
validated descriptor, proves the endpoint first, and forwards a bounded,
authenticated request second.

### Manifest linker scope

Wrong:

```rust
println!("cargo:rustc-link-arg=/MANIFESTINPUT:fyagent-test.manifest");
println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
```

This sends test-manifest state to application binaries and then tries to
disable the application manifest globally.

Correct:

```rust
println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:fyagent-test.manifest");
```

Only test targets receive the ordinary-user manifest; the formal application
binary keeps the manifest resource selected and emitted by `tauri-build`.
