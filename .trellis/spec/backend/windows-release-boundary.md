# Windows Formal Release and Runtime Activation Boundary

## 1. Scope / Trigger

This contract applies to the Windows formal release build, MSI packaging and
directory admission, early startup, single-instance activation forwarding, and
all renderer-facing commands that could probe or invoke user CLI tools. It is
mandatory because a manifest selection, MSI directory policy, elevation
decision, named-pipe endpoint, and pre-Tauri startup path form one cross-layer
privilege boundary.

It applies equally to the native Windows x64 and ARM64 GitHub Actions jobs.
Each runner builds, bundles, and verifies its own architecture; no local
non-Windows build path is part of the formal release boundary.

It distinguishes a distributable formal release from development and test
artifacts. A release-profile test harness must remain test-manifest based;
being compiled with the `release` profile alone is not evidence that a binary
may require elevation.

## 2. Signatures

```text
FYAGENT_WINDOWS_MANIFEST = release | test | dev
FYAGENT_INSTALLER_ACTIONS_DLL = target-specific installer-actions DLL
TAURI_FYAGENT_INSTALLER_ACTIONS_DLL = WiX/Tauri-visible form of that DLL path
FyAgentPureUninstall = private Type 1-derived INSTALLDIR removal state
```

```powershell
./scripts/release/verify-windows-msi.ps1 `
  -MsiPath <candidate.msi> `
  -InstallerActionsDll <validated-helper.dll> `
  -Architecture <x64|arm64> `
  -AppVersion <X.Y.Z>
```

```text
cargo:rustc-link-arg=/MANIFEST:EMBED
cargo:rustc-link-arg=/MANIFESTINPUT:<fyagent-test.manifest>
cargo:rustc-link-arg-bins=/MANIFEST:NO
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
- Each native Windows Release job exports `FYAGENT_WINDOWS_MANIFEST=release`
  on both the actual application build and MSI bundle steps. An earlier check
  is insufficient, and the variable must not be left unset or changed to
  `test` for a distributable candidate.
- Each MSI bundle invocation uses Tauri `--verbose`, captures the native exit
  code immediately, and stops before MSI enumeration when bundling fails.
  Candle/Light stderr and normal ICE validation remain enabled; `-sval` and
  individual ICE suppression are forbidden.
- Local development, build, bundle, and verification commands may target only
  the current host OS and architecture. WSL/Windows bridging, foreign
  executables, cross targets, emulators, copied toolchains, or a locally staged
  MSI do not provide release evidence. Formal Windows x64 and ARM64 installers,
  PowerShell runtime checks, Candle/Light output, and MSI table/lifecycle gates
  come only from their matching native GitHub Actions runners; no local
  cross-OS or cross-architecture candidate path exists.
- After an authorized Actions trigger, the initiating main flow waits
  synchronously for the complete run and reads final status once. Background or
  asynchronous monitoring agents and frequent polling are forbidden; failed-job
  logs are fetched only after the completed run reports failure.
- Application version resolution and the MSI helper's workspace/package
  relationship are defined by
  [FyAgent 0.3.0 Version and Installer Contract](./fyagent-version-contract.md).
  The Release version-contract job supplies the candidate version; platform
  jobs must not recover it from package.json or tauri.conf.json.
- installer-actions is an independent Windows cdylib using the locked
  windows-sys dependency family. Build it separately for each target before
  Tauri bundling, pass the same verified file through both helper environment
  variables, and verify PE Machine, MSI summary architecture, and embedded
  Binary-stream bytes. Do not make the main application crate a cdylib or add
  Tauri to the helper.
- Variable-length `MsiGetPropertyW` and `MsiRecordGetStringW` reads use a
  writable one-`u16` stack probe with input capacity zero. A null output probe,
  an unexpected success with a nonzero reported length, or any status other
  than `ERROR_SUCCESS`/`ERROR_MORE_DATA` fails closed. Either accepted status
  with a zero reported length represents an empty value; only
  `ERROR_MORE_DATA` with a positive length permits allocation and a second
  read. Property reads retain their bounded retry and record-table fields
  remain capped at 1024 UTF-16 units before the second read is accepted.
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
- `ApplyValidatedFyAgentInstallDir` is a WiX Type 35 action whose source is
  directory `INSTALLDIR` and whose target is
  `[FYAGENT_INSTALLDIR_NORMALIZED]`. It remains after `CostFinalize` only for a
  first install (`NOT Installed`, no `WIX_UPGRADE_DETECTED`, and no
  `UPGRADINGPRODUCTCODE`), so using a Type 51 `Property="INSTALLDIR"` action
  there or rewriting a maintenance target directory is invalid and must fail
  the structure gate. Repair/upgrade validates the trusted HKLM path restored
  before `CostFinalize` without applying a post-cost directory rewrite.
- Repair/upgrade clears public directory inputs, restores only the HKLM
  InstallDir anchor, and fails closed before file writes if that trusted anchor
  is unavailable. Directory validation may be skipped only for a strict pure
  uninstall whose exact `INSTALLDIR` component closure is validated from the
  rendered MSI tables. After `CostFinalize`, the architecture-matched Type 1
  helper clears private mixed-case marker `FyAgentPureUninstall`, queries the
  active Directory and Component tables, derives every `INSTALLDIR` descendant,
  and calls `MsiGetComponentStateW` on each component. It sets the marker only
  when every action state is `INSTALLSTATE_ABSENT`. The same entry point runs
  independently in the UI and Execute sequences before any marker consumer;
  the non-Handlebars UI fragment consumes only that result. The classifier
  rejects empty, duplicate, over-limit, malformed-parent, or missing-core data,
  closes database/view/record handles, and aborts closed on every query or API
  failure. The closure must contain `CMP_UninstallShortcut`,
  `InstallDirectoryAcl`, `Path`, and `RegistryEntries` and automatically covers
  every rendered resource, bundled-binary, and conditional update-task
  component beneath `INSTALLDIR`. The private marker has no Property-table
  default, and no generated component-state expression is placed in the
  255-character Sequence Condition column.
- Desktop and Start Menu shortcuts are genuinely machine-scoped while using
  the standard MSI directory identifiers. `InstallScope="perMachine"` produces
  the Property-table default `ALLUSERS=1`, which redirects `DesktopFolder` and
  `ProgramMenuFolder` to the All Users locations. Because an `msiexec` caller
  can override public properties, distinct unconditional Type 51 actions
  `EnforceFyAgentAllUsers` and
  `EnforceFyAgentDisableAdvertisedShortcuts` reassert `ALLUSERS=1` and
  `DISABLEADVTSHORTCUTS=1` in both UI and Execute sequences before
  `CostInitialize`; one sequence, a conditional action, or Property defaults
  alone are insufficient. Both shortcuts are authored as advertised children of
  the existing `Path` file/component, with no explicit target and with `Path`
  as that component's file KeyPath. Neither row authors an explicit Icon; the
  executable key file supplies it without duplicating the application binary
  into the Icon stream. Their rendered `Shortcut.Target` values must be the
  same Feature, and `FeatureComponents` must prove that Feature owns `Path`.
  `DISABLEADVTSHORTCUTS=1` in the Property table requires Windows Installer to
  create ordinary shortcuts instead of install-on-demand entry points. This
  avoids standalone profile-scoped shortcut components and their marker
  values. `RemoveShortcuts` must precede `RemoveFiles`; uninstall must not
  attempt to remove either redirected root. The only shortcut-folder cleanup
  row has no `FileName`, belongs to `Path`, and removes
  `ApplicationProgramsFolder`, the FyAgent product subdirectory beneath
  `ProgramMenuFolder`.
- `mise run` does not auto-install missing tools or provision non-host Rust
  targets. Repository tasks, scripts, and hooks never change mise trust state.
  Release targets are installed explicitly by the matching native Actions job.
- `fyagent-test.manifest` uses the two generic `cargo:rustc-link-arg` forms
  above so library unit-test and integration-test executables receive the
  Common Controls v6 activation context. Cargo 1.97.1's internal
  `LinkArgTarget::Test` selection checks `target.is_test()`; a library target's
  unit-test harness is therefore not covered by `cargo:rustc-link-arg-tests`.
  Narrowing these arguments to the tests-only form can leave Tauri/rfd's
  statically imported `TaskDialogIndirect` unavailable and make the Windows
  loader terminate the harness with `0xc0000139` before any test runs.
- The generic test-manifest inputs are paired with
  `cargo:rustc-link-arg-bins=/MANIFEST:NO`. This disables linker-generated
  manifests for application binary targets, so the test input is not merged
  into the application manifest. The application instead retains the selected
  test or release resource emitted by `tauri-build`; formal Release jobs prove
  the resulting executable with the Windows manifest verifier before and after
  MSI bundling. Generic inputs without the bin-only cancellation are invalid.
- The release manifest is `requireAdministrator`; the test manifest is
  `asInvoker`. The unsigned v0.3.0 workflow explicitly selects `release` and
  verifies the embedded application manifest in the target release executable
  before and after MSI bundling, including exact x64/ARM64 PE Machine.
- The manifest verifier never depends on `mt.exe` being present on `PATH`.
  It enumerates version directories only below the standard Windows SDK
  `Windows Kits\10\bin` roots, selects the newest architecture-matched `x64`
  or `arm64` tool by parsed SDK version, and invokes that resolved absolute
  path. A missing architecture-matched SDK tool is a release failure; PATH
  fallback, opposite-architecture fallback, or downloading a verifier is
  forbidden. The tool reads `RT_MANIFEST` resource ID 1 and never executes the
  application.
- Final MSI payload admission is read-only: the verifier resolves the unique
  File key `Path`, reads its embedded cabinet through `_Streams`, asks system
  `expand.exe` to extract only that fixed key into a fresh temporary root, and
  rejects extra/reparse/escaped output. The extracted executable must equal the
  verified built executable in File-table size, SHA-256 and PE Machine and must
  remain Authenticode `NotSigned`. It never invokes `msiexec` or an MSI action.
- FyAgent v0.3.0 deliberately ships without Authenticode. The native workflow
  requires both `fyagent.exe` and the final MSI to report `NotSigned` with no
  signer or timestamp certificate. Signing commands, certificate secrets, and
  a Release environment are forbidden in this version; future signing requires
  a separate task and decision.
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

| Condition                                                                                                                           | Required result                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `FYAGENT_WINDOWS_MANIFEST` is invalid                                                                                               | Build fails with the accepted values.                                                                                                    |
| Formal release uses release profile without an explicit manifest selection                                                          | Build fails; it never silently chooses elevated or test behavior.                                                                        |
| A native Windows Release build or bundle omits `FYAGENT_WINDOWS_MANIFEST=release`                                                   | `build.rs` fails before linking; do not weaken the fail-closed selection.                                                                |
| Test manifest uses only `cargo:rustc-link-arg-tests`, omits either generic manifest input, or omits bin-only `/MANIFEST:NO`         | Reject the change; unit/integration harnesses need the generic pair, while application bins must retain only the `tauri-build` resource. |
| Formal release process is non-elevated, not a local administrator, lacks a privilege status, or does not match the interactive user | `early_windows_startup_gate` returns `Blocked` with the appropriate safe code before Tauri construction.                                 |
| Development/test process is non-elevated                                                                                            | It may continue under the `asInvoker` test manifest.                                                                                     |
| Runtime root/state/lease has unexpected owner, DACL, object type, path, or reparse point                                            | Treat it as unavailable/untrusted; do not read, delete, or use it for activation.                                                        |
| Descriptor references a running owner but pipe open/handshake/proof fails                                                           | Do not send argv; return the activation-forward failure outcome.                                                                         |
| Authentication HMAC, frame shape, client identity, or bounds check fails                                                            | Reject the connection without invoking the activation handler.                                                                           |
| Formal release reaches a user CLI command                                                                                           | Return the elevated-boundary message before probing or running the CLI.                                                                  |
| Target executable manifest inspection or EXE/MSI `NotSigned` validation fails in release workflow                                   | Fail the workflow before publishing the artifact.                                                                                        |
| Fixed-key cabinet extraction, output containment, built-EXE SHA/size/Machine binding, or extracted unsigned check fails             | Fail the workflow; do not upload the MSI artifact.                                                                                       |
| Helper architecture, MSI Binary bytes, Type 35 assignment, machine shortcut scope, or native INSTALLDIR classifier drifts           | Fail native release structure verification before candidate artifact upload; never suppress ICE.                                         |
| UI validator rejects a path                                                                                                         | Show the recoverable policy dialog; do not raise Error 1720.                                                                             |
| Execute validator rejects a path or repair/upgrade lacks its HKLM anchor                                                            | Type 19 aborts before InstallValidate/InstallFiles.                                                                                      |

## 5. Good / Base / Bad Cases

- Good: Both native Windows Release jobs explicitly use
  `FYAGENT_WINDOWS_MANIFEST=release`, build an architecture-matched helper,
  extract its real MSI Binary stream, and prove PE/SHA-256 equality; test
  harnesses receive the generic `asInvoker`/Common Controls v6 linker input,
  application bins receive `/MANIFEST:NO`, and `tauri-build` supplies the
  selected application resource. Light retains normal ICE validation, the
  normalized directory is applied by Type 35 only on first install, the native
  post-cost classifier derives the private pure-uninstall marker, and advertised-authored
  Path-component rows plus `DISABLEADVTSHORTCUTS=1` create ordinary shortcuts
  in the machine-context standard folders.
  Startup proves the elevated process is the
  interactive local administrator, and a second invocation proves the server
  endpoint before forwarding bounded deep-link argv.
- Base: A normal developer build or test harness uses
  `FYAGENT_WINDOWS_MANIFEST=test` (or `dev`) and retains ordinary-user startup
  semantics with fake/platform-neutral tests.
- Bad: Leaving the formal Release manifest unset, narrowing the test manifest
  to `cargo:rustc-link-arg-tests`, omitting the bin-only `/MANIFEST:NO`, tying
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
- `tests/releaseWorkflow.test.ts` asserts explicit release
  manifest selection, target-executable manifest checks, MSI structure/payload
  verifiers, fixed-key cabinet extraction/built-EXE binding, and EXE/MSI
  `NotSigned` verification in the release workflow.
  It must also bind the native Release manifest selection to the actual build
  and bundle environments, require both generic test-manifest linker arguments
  plus bin-only `/MANIFEST:NO`, and reject the tests-only linker form that
  misses library unit-test harnesses.
- It must also assert the native helper build precedes bundling, architecture
  bridge variables and MSI Binary checks are present, UI/Execute Type 1 plus
  Type 19 paths are scheduled, the normalized directory assignment is Type 35,
  protected HKLM maintenance restoration is present, the native classifier
  derives the complete rendered INSTALLDIR closure with the four required core
  components and every resource/binary/conditional descendant, has no authored
  private-marker default or over-limit Sequence condition, and precedes every
  consumer in both sequences. It also checks exact InstallDir-dialog event
  order, case-sensitive closure identifiers, the shortcuts' common Path-owning
  Feature, the two exact Type 51 property-enforcement actions in both sequences
  before `CostInitialize`, `RemoveShortcuts < RemoveFiles`, and the one
  product-subdirectory cleanup row. Shortcut Feature identifiers are validated
  before entering an MSI SQL query. Directory and Component scans mirror the
  helper's 4096/32768 row caps and 1024-UTF-16-unit field cap, return only
  primitive copies, and close/final-release every COM view and record in
  `finally`. Advertised-shortcut Feature keys use the MSI Identifier grammar
  and its 38-character primary-key limit before entering SQL. Static tests also
  reject null-buffer MSI string probes. Both
  shortcuts are machine-wide Path-component rows that become ordinary
  shortcuts, standard roots are not removed, and the retired script/WMI
  validator is absent.
- The native x64 and ARM64 jobs must run the MSI verifier against the real
  bundle and the helper built earlier in that same job. The verifier checks
  ProductName, ProductVersion, ARPNOREPAIR, protocol/payload tables, summary
  architecture, Linux/retired-host residue, and the extracted
  `Binary.FyAgentInstallerActions` PE machine, length, and SHA-256. Static
  workflow checks and non-Windows workspace checks do not replace that native
  execution.
- Run the declared safe checks through mise, including Rust format/clippy and
  the targeted/unit frontend suite. Native UAC, registry, MSI, PackageManager,
  unsigned Authenticode status, and live named-pipe validation are separate
  Windows release validation and require explicit authorization; do not
  fabricate them from a non-Windows host.
- Native Windows acceptance additionally covers the default directory, a safe
  custom directory, an unsafe directory, /qn INSTALLDIR, repair, upgrade,
  uninstall, verbose MSI logging, and unsuppressed ICE validation for x64 and
  ARM64. ICE warnings must be recorded separately from ICE errors. The
  native table and Binary-stream checks are necessary structure evidence, not
  an equivalent lifecycle result. The read-only cabinet payload binding is
  mandatory package evidence but still does not replace lifecycle install
  acceptance. It must bind final MSI executable bytes to the already
  manifest-verified built executable before treating that payload as
  release-validated.

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
println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:fyagent-test.manifest");
```

This looks narrower, but Cargo 1.97.1 applies the tests-only selector only when
the target itself is marked as a test. It misses the library unit-test harness,
which can then fail in the Windows loader before test execution.

Correct:

```rust
println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
println!("cargo:rustc-link-arg=/MANIFESTINPUT:fyagent-test.manifest");
println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
```

The generic pair reaches unit and integration harnesses. The bin-only switch
prevents the application linker from generating or merging that test manifest;
the formal application binary keeps the release resource selected and emitted
by `tauri-build`, and the Release workflow verifies the final embedded resource.
