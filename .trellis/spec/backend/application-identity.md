# Application Identity Contract

## 1. Scope / Trigger

Read this contract before changing the application name, package metadata,
bundle identifiers, protocol handlers, persistence roots, exported file
markers, synchronization namespaces, autostart registration, release artifact
names, or public installation instructions. These values form one cross-layer
identity and must move together.

The 2026 FyAgent identity change is an intentional clean break from the former
application identity. It does not migrate, alias, discover, import, or clean up
the former application's local state. This boundary prevents an incomplete
compatibility layer from making old and new state appear interchangeable.

Repository provenance is a separate contract: the repository is
`NongHua123/fyagent`. Real repository URLs, release links, historical facts,
licenses, and upstream attribution must remain factual even when they contain
the former name. Never invent a FyAgent domain, repository, package listing,
or attribution to make text look uniform.

## 2. Signatures

The active application identity is:

| Surface                                                 | Required value                           |
| ------------------------------------------------------- | ---------------------------------------- |
| Product display name and autostart entry                | `FyAgent`                                |
| npm package, Cargo package, executable, portable binary | `fyagent` / `fyagent.exe`                |
| Rust library target and call site                       | `fyagent_lib` / `fyagent_lib::run()`     |
| Tauri, macOS, and Flatpak identifier                    | `com.fyagent.desktop`                    |
| Deep-link scheme                                        | `fyagent://`                             |
| Application state root                                  | `~/.fyagent`                             |
| Database and application log                            | `fyagent.db` / `logs/fyagent.log`        |
| Application-owned environment variables                 | `FYAGENT_*`                              |
| Default WebDAV/S3 root                                  | `fyagent-sync`                           |
| Skill storage serialized value                          | `fyagent`                                |
| Renderer-owned storage namespaces                       | `fyagent-*`, `fyagent.*`, or `fyagent:*` |
| Codex official-proxy marker                             | `fyagent-official`                       |
| Codex generated catalog                                 | `fyagent-model-catalog.json`             |
| Flatpak manifest, desktop, and metainfo stem            | `com.fyagent.desktop`                    |

The source repository and public source/release links remain:

```text
repository: NongHua123/fyagent
source:     https://github.com/NongHua123/fyagent
releases:   https://github.com/NongHua123/fyagent/releases
```

## 3. Contracts

### Runtime and persistence

- `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, Tauri
  configuration, platform metadata, Flatpak metadata, release workflows, and
  runtime call sites use the identity table above.
- `get_app_config_dir()` resolves the active application root directly to
  `~/.fyagent` (or its explicit test override). It must not probe the former
  application directory or use it as a fallback.
- The application opens only `fyagent.db` and emits the FyAgent SQL export
  header. Import validation does not accept an export solely because it bears
  the former application's header.
- Application-owned test and diagnostic environment variables use the
  `FYAGENT_` prefix. Existing external tool variables such as `CODEX_HOME`,
  `CLAUDE_CONFIG_DIR`, or provider API variables retain their upstream names.

### OS integration and serialization

- Only `fyagent://` is registered and dispatched. The former scheme is neither
  registered nor parsed as an alias.
- The OS sees FyAgent as a distinct application through
  `com.fyagent.desktop`, the `fyagent` executable, and the `FyAgent` autostart
  entry. Do not remove or rewrite a former installation's registration as a
  side effect of launching FyAgent.
- Persisted application-owned enums, sync roots, local-storage keys, proxy
  markers, reasoning/tool sentinels, generated Codex projections, and release
  artifact names use FyAgent-owned values. Do not add dual-read or dual-write
  behavior for former identity values.

### Public text and provenance

- User-facing current-product text says FyAgent, but links to source, issues,
  releases, and contribution history use the real repository location.
- Historical changelogs, release notes, design baselines, copyright notices,
  licenses, and upstream issue references remain historically accurate, except
  that commercial campaign material is removed under the repository's current
  product-content policy.
- Commercially attributed URLs and attached tracking query data are not
  provenance or compatibility requirements. Remove them rather than carrying
  them into runtime configuration, public documentation, or release history.
- Installation documentation may advertise only distribution channels that
  actually exist. Do not infer `fyagent.io`, a renamed GitHub repository,
  Homebrew cask, AUR package, Pages deployment, signing, or notarization.

## 4. Validation & Error Matrix

| Condition                                                                                                                                  | Required result                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Former app identifier, scheme, data root, database name, executable, or owned serialization marker appears in active runtime/configuration | Reject the change or classify and remove the active dependency.                                                             |
| A former installation or data directory exists                                                                                             | FyAgent ignores it and starts with independent state; no automatic migration or cleanup.                                    |
| A former deep link is opened                                                                                                               | It is not registered or accepted as a FyAgent import.                                                                       |
| A current release workflow expects the former executable or bundle ID                                                                      | Static release tests or CI fail before publication.                                                                         |
| A real repository URL, LICENSE line, historical record, or upstream reference contains the former name                                     | Preserve it and record it as an evidence-backed exception; do not cosmetically rewrite it.                                  |
| Runtime configuration, public documentation, or release history contains commercial campaign material                                      | Remove it; it does not establish an identity or provenance exception.                                                       |
| Public text points to an unverified FyAgent domain, repository, package manager entry, signature, or notarization claim                    | Remove the claim and link to the verified repository/release surface instead.                                               |
| The identity changes on only one layer                                                                                                     | Reject as incomplete; verify package, Rust, Tauri, OS integration, storage, UI, tests, docs, and release workflow together. |
| Static checks pass but native installation/launch was not exercised                                                                        | Report native acceptance as pending; do not claim installation, signing, or upgrade compatibility.                          |

## 5. Good / Base / Bad Cases

- Good: a fresh install registers `fyagent://`, stores state in `~/.fyagent`,
  launches the `fyagent` binary under `com.fyagent.desktop`, and produces
  FyAgent-named artifacts while source links still point to the real
  `NongHua123/fyagent` repository.
- Base: the machine also contains the former application and its data. FyAgent
  leaves both untouched and presents independent fresh state.
- Bad: startup checks the former data directory, accepts the former URL scheme,
  deletes a former autostart entry, changes LICENSE attribution, or links
  users to an assumed FyAgent website.

## 6. Tests Required

- Rust configuration tests assert the FyAgent root, database, log, export
  header, environment override, sync root, Skill serde value, Codex-owned
  markers, and the absence of former-identity fallback reads.
- Deep-link tests assert `fyagent://` parsing and platform registration; a
  former-scheme fixture must be rejected or remain unregistered.
- TypeScript tests assert current local-storage keys, displayed product name,
  current serialized values, and FyAgent API examples.
- Packaging/release tests assert `fyagent` / `fyagent.exe`,
  `com.fyagent.desktop`, `FyAgent.app`, current Flatpak filenames, and the
  expected artifact names on every platform.
- Static identity audits classify every former-name hit. Active code and
  current instructions must have none unless the token is an external fact;
  negative assertions, repository URLs, history, legal attribution, upstream
  references are reviewed exceptions.
- Parse changed JSON, plist/XML, TOML, YAML, and locale files. Run format,
  type-check, unit/integration, Rust, and platform packaging checks in the
  authorized environment; keep native install/launch and signed/notarized
  release acceptance separate.

## 7. Wrong vs Correct

### Wrong

```rust
// A fallback silently turns the new identity into an incomplete migration.
let root = find_existing(".cc-switch").unwrap_or(home.join(".fyagent"));
```

```text
# A blind text replacement corrupts facts outside application identity.
https://github.com/NongHua123/fyagent
```

### Correct

```rust
// FyAgent owns one independent state root.
let root = home.join(".fyagent");
```

```text
# Product identity and repository provenance are deliberately independent.
product:    FyAgent
repository: https://github.com/NongHua123/fyagent
```
