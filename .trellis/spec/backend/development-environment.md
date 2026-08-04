# Development Environment Contract

## 1. Scope / Trigger

Read this contract before changing development tool versions, onboarding
instructions, local build/test commands, WSL scripts, or any workflow that
selects repository development tools. It applies to local development on
Windows, macOS, Linux, and WSL. CI and release runners retain their explicit
platform setup until a separately validated migration changes them.

## 2. Signatures

The local development environment is declared by:

```text
mise.toml  human-reviewed tool versions, options, targets, and tasks
mise.lock  generated download URLs and checksums where supported
```

Developers install one global mise binary. The repository requires mise
`>= 2026.8.0`; project scripts do not download or privately install mise.

The active versions are:

```text
Node.js 22.12.0
pnpm    10.12.3
Python  3.12.8
Rust    1.95.0 + rustfmt + clippy
```

The Rust definition also provisions `aarch64-apple-darwin` and
`x86_64-apple-darwin` for the WSL macOS cross-build workflow. Tauri CLI is a
project dependency installed by pnpm rather than a separately managed global
tool.

## 3. Contracts

- The user-installed global mise is the required version manager and command
  environment for local development. After reviewing the repository config,
  run `mise trust` once and `mise install` whenever the declared tools change.
- Copy-pasteable project commands must use `mise exec -- <command>` so they do
  not depend on shell activation. An unprefixed `node`, `pnpm`, `python`,
  `rustc`, or `cargo` command is valid only in a shell where mise is activated
  or its shims are intentionally configured.
- `mise.toml` is the primary local tool-version declaration. `.node-version`,
  `package.json#packageManager`, and `rust-toolchain.toml` are compatibility
  inputs for their ecosystems and must resolve to the same versions.
- `mise.lock` targets `linux-x64`, `linux-arm64`, `macos-x64`, `macos-arm64`,
  `windows-x64`, and `windows-arm64`, recording URLs and checksums for each tool
  where its mise backend publishes a lockable artifact. Do not hand-edit it.
  After an intentional version change, regenerate all target platforms with:

  ```bash
  mise lock --platform linux-x64,linux-arm64,macos-x64,macos-arm64,windows-x64,windows-arm64
  ```

- mise manages language runtimes and portable development tools. The host
  package manager remains responsible for native compilers, linkers, headers,
  WebView libraries, and other platform prerequisites required by Tauri.
- A tool-version change must update all compatibility declarations, regenerate
  `mise.lock`, update affected documentation, and pass the environment
  consistency tests in the same change.
- Commands executed in WSL must not resolve managed tools from `/mnt/<drive>`
  or a Windows shim. Fix PATH or invoke `mise exec`; do not work around
  UNC/cmd.exe failures with copied dependencies.
- Project scripts must not download a second mise binary or override
  `MISE_DATA_DIR`, `MISE_CACHE_DIR`, `MISE_STATE_DIR`, `MISE_CARGO_HOME`,
  `MISE_RUSTUP_HOME`, `CARGO_HOME`, or `RUSTUP_HOME` to create a private mise
  installation. The WSL macOS workflow resolves the global mise path before
  PATH isolation, then invokes that absolute binary for `mise install` and
  `mise exec`.
- GitHub Actions CI and release jobs are not local onboarding paths. Preserve
  their explicit runner setup, caching, signing, and target provisioning until
  a dedicated migration validates every runner architecture and release path.
- Filesystem tests that require a write to fail must create a deterministic
  invalid shape inside the isolated test root, such as using a regular file as
  the requested target's parent. Do not infer failure from a fixed absolute
  "nonexistent" path or permission bits: root, elevated Windows accounts, and
  container mounts can make those paths writable. Shared fixture locks must
  recover poisoning when every holder resets the fixture after acquisition, so
  one assertion failure does not hide the root cause behind cascading lock
  failures.

## 4. Validation & Error Matrix

| Condition                                                                       | Required result                                                                        |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Global mise is missing or older than 2026.8.0                                   | Stop before installing project dependencies or running development commands            |
| A managed command resolves outside mise in a non-activated shell                | Re-run through `mise exec` or correctly activate mise                                  |
| A WSL managed command resolves under `/mnt`                                     | Stop and repair PATH; never invoke the Windows shim                                     |
| `mise.toml` differs from `.node-version`, packageManager, or Rust toolchain     | Fail the consistency test                                                              |
| A configured platform is absent from all applicable `mise.lock` tool entries    | Regenerate the full target list; document any backend artifact gap before claiming support |
| A native library is unavailable through mise                                    | Install and document the minimum host package; do not add another runtime manager       |
| Existing global mise has different tools installed                              | Project `mise.toml` wins; no globally selected tool version is assumed                  |
| A project script downloads mise or configures private mise/Cargo/rustup homes    | Reject the change and reuse the user-installed global mise                              |

## 5. Good / Base / Bad Cases

- Good: install global mise once, review and trust the repository config, run
  `mise install`, then use `mise exec -- pnpm test:unit` and
  `mise exec -- cargo test --manifest-path src-tauri/Cargo.toml`.
- Base: configure mise shell activation for interactive work and run the same
  unprefixed commands after verifying that `mise which` owns each managed tool.
- Base: install a documented native compiler or header with the host package
  manager while keeping language runtimes under mise.
- Bad: install arbitrary Node/Rust versions independently, use a Windows pnpm
  shim from WSL, or bootstrap a private mise copy for one workflow.

## 6. Tests Required

- Parse `mise.toml`, `.node-version`, `package.json`, and
  `rust-toolchain.toml`; assert equivalent exact versions.
- Assert `mise.lock` contains applicable entries for every configured target
  platform and the expected Rust tool/options entry.
- Run `mise install`, then verify `node`, `pnpm`, `python`, `rustc`, `cargo`,
  rustfmt, clippy, and required Rust targets from inside `mise exec`.
- Verify maintained onboarding and quality-gate documents identify mise as the
  local command environment and use explicit `mise exec` examples.
- In WSL packaging tests, compare `command -v` with global `mise which` and fail
  if a managed command differs or resolves below `/mnt`.
- Assert WSL macOS scripts contain no mise download URL/checksum and do not
  export private mise, Cargo, or rustup home variables.
- Run filesystem failure-path integration tests under an elevated/root context
  as well as an ordinary user context; both must fail at the intended path
  shape boundary without creating an absolute host directory, and a deliberately
  poisoned shared fixture lock must be recoverable by the next holder.

## 7. Wrong vs Correct

Wrong: follow broad minimum-version prose, run whichever Node/Rust tool happens
to be on PATH, or assume a passing command used the repository toolchain.

Correct: install and trust global mise once, install the versions declared by
the repository, run project commands through `mise exec` (or a verified
activated shell), and treat missing native libraries as explicit host
dependencies.
