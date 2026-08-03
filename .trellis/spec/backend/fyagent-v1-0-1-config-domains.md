# FyAgent v1-0.1 Configuration Domains and Version Contract

## 1. Scope / Trigger

Read this contract before changing the FyAgent application version, the Codex
provider-native capabilities, the Codex Desktop restart flow, or WorkBuddy
configuration. These changes cross Rust/Tauri commands, renderer state,
user-owned configuration files, and Windows process identity boundaries.

The product baseline is `docs/fyagent/dev/v1-0.1/`. Those documents contain a
historical `3.18.0` source snapshot and must not be mechanically rewritten as
part of an application-version update. WorkBuddy is a top-level configuration
domain, not an `AppType`, Provider, MCP, Skill, Prompt, Profile, Session, usage,
or local-proxy domain.

## 2. Signatures

### Independent application version

```text
package.json.version                = 0.1.0
src-tauri/Cargo.toml package.version = 0.1.0
src-tauri/tauri.conf.json.version    = 0.1.0
```

`Cargo.lock` is refreshed through Cargo after the manifest change. The local
static test is `tests/versionConsistency.test.ts`; it parses the three metadata
sources and rejects non-SemVer or divergent values. Release workflows, tags,
updaters, changelogs, and historical documentation are deliberately outside
this version-chain contract unless a later task explicitly expands it.

### Codex provider and restart IPC

```text
add_provider_with_result(provider, app, addToLive?)
update_provider_with_result(provider, app, originalId?)
delete_provider_with_result(id, app)
switch_provider_with_result(id, app)
import_default_config_with_result(app)
  -> { value, liveConfigChanged, app }

analyze_codex_provider_features(app: "codex", provider, isNew?)
  -> CodexProviderFeatureState
patch_codex_provider_features(app: "codex", provider, intent, isNew?)
  -> { tomlText, state, imageExtensionConfigured? }

get_codex_desktop_runtime_status()
request_codex_desktop_restart()
continue_codex_desktop_restart_with_force(token)
```

The feature commands reject every `app` other than Codex. The restart commands
accept no PID, process name, executable path, or user-supplied launch command.
The force token is opaque, short lived, one-time, and bound server-side to the
already verified installation and process instance.

### WorkBuddy IPC

```text
get_workbuddy_status() -> WorkBuddyStatus
fetch_workbuddy_models(FetchWorkBuddyModelsRequest)
  -> { models: string[], truncated: boolean }
save_workbuddy_models(SaveWorkBuddyModelsRequest)
  -> { revision, modelCount, createdEntries, updatedEntries, duplicateIds }
```

`FetchWorkBuddyModelsRequest` is `{ baseUrl, apiKey, allowNoApiKey }`.
`SaveWorkBuddyModelsRequest` additionally carries selected/manual IDs,
`clearExistingApiKeys`, an opaque `expectedRevision`, and optional
`duplicatePolicy: "reject" | "updateAll"`. These dedicated commands do not
accept `AppType`, Provider IDs, or renderer-controlled filesystem paths.

## 3. Contracts

### Codex native capabilities and live result

- Determine capability eligibility from the actual editable third-party
  provider TOML table, base URL, and ordinary API credentials; do not infer it
  from a provider-type string. Official, managed-account/OAuth, read-only, and
  incomplete providers are ineligible.
- Read and patch the form TOML using `toml_edit`. Preserve comments, blank
  lines, table/field order, unrelated fields, and unrelated headers.
- The image capability controls only a case-insensitive
  `x-openai-actor-authorization` header whose value is exactly
  `local-image-extension`. A conflicting same-name header is protected: do not
  overwrite, delete, or create a differently cased duplicate. Saving unrelated
  provider fields may preserve such a conflict, but an explicit image-toggle
  patch must fail closed until the user resolves it in TOML.
- The WebSocket capability writes `supports_websockets = true` only for
  `meta.apiFormat == "openai_responses"`. Disabling removes the field, rather
  than writing `false`. Moving to another upstream format removes an already
  enabled field before save; a manually restored incompatible `true` blocks
  save.
- `ProviderMeta.imageExtensionConfigured` is migration-only private metadata.
  Missing metadata plus no managed/conflicting header is a legacy pending-on
  draft; no bulk upgrade may write live TOML. A successful first new-provider
  save or explicit historical decision marks the row configured. UI state still
  derives from TOML, not this marker alone.
- `liveConfigChanged` means only that a successful operation changed the final
  bytes of this user’s `~/.codex/config.toml`. It contains neither bytes,
  content hashes, paths, nor credentials. Non-Codex mutations return `false`.

### Trusted Codex Desktop restart

- The renderer offers a restart prompt only after a successful Codex mutation
  reports `liveConfigChanged: true` and the backend reports exactly one trusted
  running instance. Saving configuration and restarting are separate results;
  a failed/cancelled restart never rolls back the saved configuration.
- Windows identifies processes through the previously verified package
  identity; macOS matches the verified bundle identity and path. Do not use
  fuzzy executable/process-name matching or expose a generic kill command.
- Request graceful exit and wait at most 8 seconds. If it is still alive, the
  backend returns an opaque force-confirmation token. Only a second explicit
  user confirmation may force termination. Launch only after the old verified
  instance has exited, through the originally selected verified installation,
  then wait at most 15 seconds for that same trusted installation’s new
  instance. Installation or identity drift is a no-launch failure, not an
  opportunity to select a different candidate.
- Not-running, unsupported, ambiguous, later/manual choice, and restart
  failure must not auto-launch any process.

### WorkBuddy fetch and persistence

- Read/write only the current user’s `~/.workbuddy/models.json` (or the
  existing `FYAGENT_TEST_HOME` test-home override), with exactly one same-folder
  backup `models.json.backup`. Never probe `.codebuddy`, project paths, or the
  real profile in tests.
- Normalize only absolute HTTP(S) URLs with a host, no userinfo/query/fragment;
  strip only terminal `/models`, `/chat/completions`, or `/responses`, then
  append `/v1` if no decoded path segment equals `v1`. Request exactly
  `<normalized-base>/models`.
- Use a short-lived restricted client: 15-second total deadline, manual maximum
  three same-origin redirects, no HTTPS downgrade, 2 MiB streamed response
  limit, and no Authorization header when the user explicitly allows an empty
  key. A nonempty key is sent only to the original/validated same-origin URL.
- Parse only an object containing `data: []`; every element must have a
  nonempty string `id`. Preserve upstream order and case-sensitive first
  occurrences. Return at most 1,000 IDs and `truncated: true` if a valid
  1,001st unique ID exists; continue validating the remaining bounded body so
  truncation cannot mask a malformed element.
- A save takes the in-process write lock, rereads the current bytes, checks the
  opaque revision, validates every existing array object/ID, detects duplicate
  target IDs, and only then writes. `reject` returns duplicate IDs/counts with
  no backup and no main-file write; the UI freezes the exact request and retries
  only it with `updateAll`. The backend must validate revision again.
- The externally returned revision is a process-local-key HMAC of the complete
  current file bytes, not a bare digest. It therefore detects even an external
  API Key-only change without letting the renderer validate Key guesses against
  a public file hash. Never persist or serialize the HMAC key; after a host
  restart an old token must fail safely and the renderer refreshes status.
- Preserve non-target entries, array order, target positions, unknown fields,
  and unknown `reasoning` fields. Update only documented managed fields and
  remove `onlyReasoning`. Write backup then primary by flush/sync plus
  same-directory atomic replacement; Windows must use replacement semantics
  without a delete-before-rename path, Unix files must remain `0600`.
- API keys may enter only component memory and a Tauri request, never
  localStorage/sessionStorage/query cache/log/error DTO. The on-disk primary
  and backup files are credential files and receive the same protection.

### Renderer domain boundary

`TopLevelAppId = AppId | "workbuddy"`; `AppId` itself remains the provider
domain type. WorkBuddy follows Codex and precedes Gemini in the switcher.
Missing legacy `visibleApps.workbuddy` resolves to `true`. Entering WorkBuddy
mounts only its status/configuration surface and does not invoke provider,
current-provider, MCP, Skills, profile, usage, environment/migration, or proxy
queries. Its API key clears on unmount and is never refilled from disk.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Metadata versions differ or are invalid SemVer | Local version consistency test fails; do not hand-edit the lockfile. |
| Non-Codex app calls a native-feature command | Command rejects before TOML analysis or patch. |
| Image header has a conflicting value or invalid header shape | Show protected/invalid state; explicit image patch fails without leaking the value. |
| Non-Responses provider saves with `supports_websockets = true` | Reject save; never persist incompatible WebSocket transport. |
| DB/provider action succeeds but live Codex bytes are unchanged | Return `liveConfigChanged: false`; do not ask to restart. |
| Several/non-identical trusted installations or running instances exist | Return ambiguous/unavailable; do not close or launch any process. |
| Graceful exit exceeds 8 seconds | Require the opaque second-confirmation token; no automatic force kill. |
| New process is absent at 15 seconds or installation drifts | Return restart failure; retain saved configuration and direct user to manual restart. |
| WorkBuddy URL is non-HTTP(S), has credentials/query/fragment, or redirect leaves origin | Return `WORKBUDDY_INVALID_URL` or `WORKBUDDY_FETCH_REDIRECT_REJECTED`; do not send credentials onward. |
| WorkBuddy response exceeds 2 MiB, times out, or has malformed `data[]` | Return bounded fetch error; retain no model IDs from that response. |
| Existing models JSON is invalid/not-array/contains an invalid entry | Return safe config error with only an index when applicable; do not repair or overwrite it. |
| Revision changes or target IDs are duplicated under `reject` | Return conflict; create no backup and write no primary. |
| WorkBuddy UI receives a truncated result | Keep the truncation warning visible until a subsequent successful non-truncated fetch replaces it. |

## 5. Good / Base / Bad Cases

- Good: An eligible third-party Codex provider with an unknown TOML header
  enables image support. Its exact managed header is added, comments and other
  headers stay byte-positioned as `toml_edit` preserves them, and a live byte
  change may prompt for a trusted restart once.
- Base: A historical provider lacks the marker and managed header. The editor
  displays pending-on, but cancelling the dialog creates no TOML/database
  migration. A later save records either the explicit enabled or disabled
  decision.
- Bad: Renderer sends `{ pid: 1234 }`, a different `app`, or a launch path to
  a restart command; a process-name scan kills `codex.exe`; the backend accepts
  any such control.
- Good: WorkBuddy fetches an ordered model response, returns the first 1,000
  unique IDs plus `truncated: true`, and a user confirms duplicate update-all
  against the same revision. Non-target JSON and extra `reasoning` keys remain.
- Base: The user explicitly allows an empty key; fetch/save sends no
  Authorization and existing per-model keys remain unless clear-existing is
  selected.
- Bad: A generic model fetcher sorts IDs, tries several endpoint suffixes,
  forwards a key to a cross-origin redirect, silently removes invalid JSON
  entries, or deletes the Windows target before rename.

## 6. Tests Required

- TypeScript: parse all three version metadata files and assert exact `0.1.0`;
  test legacy WorkBuddy visibility/order, top-level isolation, all four locale
  key sets, password/default key lifecycle, HTTP warning, persistent truncation,
  duplicate-dialog frozen request/retry, and Codex capability/restart dialogs.
- Rust Codex: TOML comments/order/unknown headers, case-insensitive managed
  header behavior, conflict/invalid shape protection, historical marker
  migration, Responses-only WebSocket validation, live-byte change truth table,
  command app guard, and fake-platform trusted restart state machine including
  graceful timeout, force confirmation, original-installation drift, and
  15-second verification failure.
- Rust WorkBuddy: URL normalization/rejection, redirection and Authorization
  policy, timeout/2 MiB bounds, malformed entries after cap, exact order and
  case-sensitive de-duplication, no-key behavior, HMAC revision opacity and
  API-Key-only revision conflict, duplicate reject/update-all, unknown-field
  preservation, backup/primary failure paths, and test-home isolation. Tests
  must not access the real profile.
- Local gates when dependencies permit: `pnpm typecheck`, `pnpm format:check`,
  `pnpm test:unit`, `pnpm run build:renderer`, `cargo fmt --check`, offline
  locked `cargo clippy -D warnings`, offline locked `cargo test`, and
  `git diff --check`. Do not characterize these as native E2E, platform, CI, or
  release evidence.

## 7. Wrong vs Correct

### Wrong

```rust
// A renderer-controlled PID turns a narrow trusted restart into a kill API.
pub fn restart(pid: u32, path: String) { terminate_process(pid); launch(path); }
```

```rust
// Deleting first can lose a credential file if the rename fails on Windows.
fs::remove_file(target)?;
fs::rename(temp, target)?;
```

```ts
// WorkBuddy is not a provider-domain application ID.
providersApi.getAll("workbuddy" as AppId);
```

### Correct

```rust
// The service retains the verified installation/instance; the renderer only
// sends an opaque continuation token after explicit user confirmation.
let outcome = service.continue_restart_with_force(opaque_token).await?;
```

```rust
// Same-directory replace preserves the old destination on replacement failure.
write_temp_and_sync(parent, bytes)?;
replace_file(temp, target)?;
```

```ts
// WorkBuddy owns a separate top-level route and dedicated IPC surface.
const active: TopLevelAppId = "workbuddy";
return <WorkBuddyPage />;
```
