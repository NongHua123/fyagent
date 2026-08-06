# Windows Formal Release and Runtime Activation Boundary

## 1. Scope / Trigger

This contract applies to the Windows formal release build, MSI packaging,
early startup, single-instance activation forwarding, and all renderer-facing
commands that could probe or invoke user CLI tools. It is mandatory because a
manifest selection, elevation decision, named-pipe endpoint, and pre-Tauri
startup path form one cross-layer privilege boundary.

It distinguishes a distributable formal release from development and test
artifacts. A release-profile test harness must remain test-manifest based;
being compiled with the `release` profile alone is not evidence that a binary
may require elevation.

## 2. Signatures

```text
FYAGENT_WINDOWS_MANIFEST = release | test | dev
```

```rust
pub(crate) const fn formal_windows_build() -> bool;
pub fn early_windows_startup_gate() -> WindowsStartupDisposition;
pub fn runtime_privilege_status() -> RuntimePrivilegeStatus;
pub(crate) fn install_activation_handler<F>(handler: F)
    -> Result<(), WindowsStartupErrorCode>;

fn elevated_windows_cli_boundary_active_for(formal_windows_build: bool) -> bool;
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
- The release manifest is `requireAdministrator`; the test manifest is
  `asInvoker`. The signed release workflow explicitly selects `release` and
  verifies the embedded application manifest before signing, after signing,
  and for the final release artifact.
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

| Condition | Required result |
| --- | --- |
| `FYAGENT_WINDOWS_MANIFEST` is invalid | Build fails with the accepted values. |
| Formal release uses release profile without an explicit manifest selection | Build fails; it never silently chooses elevated or test behavior. |
| Formal release process is non-elevated, not a local administrator, lacks a privilege status, or does not match the interactive user | `early_windows_startup_gate` returns `Blocked` with the appropriate safe code before Tauri construction. |
| Development/test process is non-elevated | It may continue under the `asInvoker` test manifest. |
| Runtime root/state/lease has unexpected owner, DACL, object type, path, or reparse point | Treat it as unavailable/untrusted; do not read, delete, or use it for activation. |
| Descriptor references a running owner but pipe open/handshake/proof fails | Do not send argv; return the activation-forward failure outcome. |
| Authentication HMAC, frame shape, client identity, or bounds check fails | Reject the connection without invoking the activation handler. |
| Formal release reaches a user CLI command | Return the elevated-boundary message before probing or running the CLI. |
| Manifest inspection/signing validation fails in release workflow | Fail the workflow before publishing the artifact. |

## 5. Good / Base / Bad Cases

- Good: The signed release workflow explicitly builds with
  `FYAGENT_WINDOWS_MANIFEST=release`; startup proves the elevated process is
  the interactive local administrator; a second invocation proves the server
  endpoint before it forwards a bounded deep-link argv.
- Base: A normal developer build or test harness uses
  `FYAGENT_WINDOWS_MANIFEST=test` (or `dev`) and retains ordinary-user startup
  semantics with fake/platform-neutral tests.
- Bad: Tying `requireAdministrator` to every release-profile binary, accepting
  a state file by path alone, exposing a fixed well-known pipe, sending argv
  before endpoint proof, or allowing a formal elevated build to shell out to
  an arbitrary user CLI.

## 6. Tests Required

- `src-tauri/src/windows_runtime/mod.rs` unit tests must cover bounded frame
  encode/decode, tamper/control/trailing-data rejection, descriptor decision
  policy, privilege-gate behavior, SDDL policy predicates, and both server and
  request HMAC proofs.
- `tests/desktopSecurityBoundary.test.ts` must assert the narrow renderer
  capability/CSP boundary, absence of portable Windows distribution claims,
  visual-baseline LFS policy, and pre-probe formal-release CLI guard.
- `tests/releaseWorkflow.test.ts` must assert explicit release manifest
  selection and manifest/signing verification steps in the release workflow.
- Run the declared safe checks through mise, including Rust format/clippy and
  the targeted/unit frontend suite. Native UAC, registry, MSI, PackageManager,
  signing, and live named-pipe validation are separate Windows release
  validation and require explicit authorization; do not fabricate them from a
  non-Windows host.

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
