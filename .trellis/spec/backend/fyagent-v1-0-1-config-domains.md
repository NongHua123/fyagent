# FyAgent v1-0.1 Configuration Domains

## 1. Scope / Trigger

Read this contract before changing the Codex provider-native capabilities, the
Codex Desktop restart flow, or WorkBuddy configuration. These changes cross
Rust/Tauri commands, renderer state, user-owned configuration files, and
Windows process identity boundaries.

Application-version, release-metadata, and MSI directory-policy changes belong
to [FyAgent 0.2.1 Version and Installer Contract](./fyagent-version-contract.md).

The v1.0.1 configuration-domain input is `docs/fyagent/dev/v1-0.1/`. For the
active v1.0.2 scope, `docs/fyagent/dev/v1-0.2/` is authoritative where it
supersedes a contract; retain this v1.0.1 contract only for configuration-domain
rules not superseded by v1.0.2. The v1.0.1 documents contain a historical
`3.18.0` source snapshot and must not be mechanically rewritten as part of an
application-version update. WorkBuddy is a top-level configuration domain, not
an `AppType`, Provider, MCP, Skill, Prompt, Profile, Session, usage, or
local-proxy domain.

## 2. Signatures

### Versioning boundary

This historical configuration-domain contract owns no application-version
literal, Cargo.lock rule, tag, release asset, or Tauri metadata field. Follow
the dedicated version and installer contract before changing those boundaries;
do not restore the historical three-field version chain in this file.

### Codex provider and restart IPC

```text
add_provider_with_result(provider, app, addToLive?)
update_provider_with_result(provider, app, originalId?)
delete_provider_with_result(id, app)
switch_provider_with_result(id, app)
import_default_config_with_result(app)
  -> { value, liveConfigChanged, app, warningCodes? }

analyze_codex_provider_features(app: "codex", provider, isNew?)
  -> CodexProviderFeatureState
patch_codex_provider_features(app: "codex", provider, intent, isNew?)
  -> {
       tomlText,
       state,
       imageExtensionConfigured?,
       codexNativeCapabilitiesGeneratedProvider?
     }

get_codex_desktop_runtime_status()
request_codex_desktop_restart()
continue_codex_desktop_restart_with_force(token)
cancel_codex_desktop_restart_with_force(token) -> ()
```

The feature commands reject every `app` other than Codex. The restart commands
accept no PID, process name, executable path, or user-supplied launch command.
The force token is opaque, short lived, one-time, and bound server-side to the
already verified installation and process instance. Cancellation is a
best-effort discard of a pending continuation capability only: it never closes,
terminates, or launches a process, and its no-result response does not reveal
whether a token, process, or installation exists.

### WorkBuddy IPC

```text
get_workbuddy_status() -> WorkBuddyStatus
fetch_workbuddy_models(FetchWorkBuddyModelsRequest)
  -> { models: string[], truncated: boolean }
save_workbuddy_models(SaveWorkBuddyModelsRequest)
  -> { state: "saved", revision, modelCount, createdEntries, updatedEntries }
   | { state: "overwrite_confirmation_required", token, existingIds }
   | { state: "concurrent_modification" }
```

`FetchWorkBuddyModelsRequest` is `{ baseUrl, apiKey, allowNoApiKey }`.
`SaveWorkBuddyModelsRequest` additionally carries selected/manual IDs,
`clearExistingApiKeys`, an opaque `expectedRevision`, and optional
`overwriteToken`. The backend issues `overwriteToken` only after detecting an
existing target and binds it to the normalized, otherwise unchanged save
request and expected revision. These dedicated commands do not accept `AppType`,
Provider IDs, or renderer-controlled filesystem paths.

## 3. Contracts

### Codex native capabilities and live result

- Every Codex Provider exposes both native-capability controls in the existing,
  initially collapsed advanced region. Provider ID, `base_url`, credentials,
  official/managed classification, OAuth type, proxy takeover, `wire_api`, and
  `meta.apiFormat` do not make a valid TOML draft ineligible. A fixed official
  Provider is identified only by `category == "official"` or ID
  `codex-official`; names, URLs, and `requires_openai_auth` are not classifiers.
- Read and patch the form TOML using `toml_edit`. Preserve comments, blank
  lines, table/field order, unrelated fields, and unrelated headers.
- An invalid complete TOML document keeps both controls visible but disabled
  and blocks capability writes. An invalid `http_headers` or
  `supports_websockets` field is a non-blocking diagnostic: ordinary saves
  preserve it, and only an explicit operation on that control repairs it.
- The image capability controls only a case-insensitive
  `x-openai-actor-authorization` header whose value is exactly
  `local-image-extension`. Enabling removes all case variants and writes one
  canonical key; disabling removes all variants and removes an empty header
  table. Other strings in a valid header map survive. When the field is not a
  string map, explicit enable replaces the entire field with the managed map
  and explicit disable deletes it; no unrelated save performs this repair.
- The WebSocket capability is format-agnostic configuration. Enabling always
  writes boolean `supports_websockets = true`; disabling removes the field
  rather than writing `false`. API-format changes neither remove nor disable
  it, and Responses, Chat, Anthropic, managed OAuth, official, and proxy
  scenarios all remain saveable. An invalid field type is overwritten or
  removed only by an explicit WebSocket-control operation.
- `ProviderMeta.imageExtensionConfigured` is migration-only private metadata.
  For non-official Providers, missing metadata plus no managed/conflicting
  header is a legacy pending-on draft; no bulk upgrade may write live TOML. A
  successful first new-provider save or explicit historical decision marks the
  row configured. UI state still derives from TOML, not this marker alone.
- Fixed official Providers default both capabilities off and do not receive a
  Provider table merely by opening or saving the form. The first actual enable
  creates `model_provider = "custom"` and a minimal table with `name =
"OpenAI"`, `requires_openai_auth = true`, and `wire_api = "responses"`.
  Private `ProviderMeta.codexNativeCapabilitiesGeneratedProvider` records
  ownership only when the capability patch actually creates the table; a
  pre-existing inactive `custom` table may be reused but is never claimed.
  When both capabilities are off, remove an owned table only if its exact
  managed shape remains and it has no user fields; otherwise remove only the
  capability fields. An explicit Provider table takes precedence over unified
  Codex session-history injection.
- A native Responses Provider receives an official vendor model catalog that
  grants freeform tools or vendor harness instructions only when its active
  `base_url` parses as HTTPS to an approved host. The current DeepSeek catalog
  permits exactly `deepseek.com` and its dot-delimited subdomains. Non-HTTPS
  schemes and URL substring, path, or user-info matches such as
  `deepseek.com.evil.example`, `notdeepseek.com`, or
  `deepseek.com@evil.example` must keep the neutral native template.
- Session resume commands cross a shell-command boundary in macOS terminal
  launchers and are copied for an unspecified user shell on other platforms.
  Every dynamic argument sourced from persisted session data must pass the
  shared fail-closed helper before command construction. It accepts only a
  nonempty ASCII identifier whose first character is alphanumeric or `_` and
  whose remaining characters are alphanumeric, `_`, `-`, or `.`. Ordinary
  UUID/provider-prefixed IDs retain their existing command text. For every
  other shape, including leading hyphens, do not generate `resumeCommand`; a
  future wider grammar requires typed argv and platform-aware launch/copy
  behavior rather than shell-string escaping.
- Successful Codex add/update mutations may return warning codes
  `CODEX_WEBSOCKET_NON_GPT_MODEL` and
  `CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED`. Compute them from the final saved
  Provider only when WebSocket is `true`. Check nonempty top-level `model`,
  `review_model`, and `modelCatalog.models[].model`; take the segment after the
  final `/` and accept an ASCII case-insensitive `gpt-` prefix. Any recognizable
  non-GPT model warns; no recognizable models do not. Active Codex proxy
  takeover adds the proxy warning. Warning codes are omitted for switching,
  failed saves, and empty-risk results.
- Normal and official proxy projections preserve explicit WebSocket and the
  managed image header while continuing to rewrite routing `base_url` and
  `wire_api`. The local proxy still exposes HTTP/SSE only; preservation and a
  warning are not a claim of WebSocket Upgrade support.
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
  target IDs, and only then writes. Existing targets without a valid matching
  confirmation capability return `overwrite_confirmation_required` with one
  opaque token and unique `existingIds`, with no backup or main-file write. The
  UI freezes the exact preflight request and retries only it with that token.
  The backend consumes the token before rereading, validates its request and
  revision binding, and validates the revision again after the reread; reused,
  malformed, mismatched, or expired tokens never authorize a write.
- The externally returned revision is a process-local-key HMAC of the complete
  current file bytes, not a bare digest. It therefore detects even an external
  API Key-only change without letting the renderer validate Key guesses against
  a public file hash. Never persist or serialize the HMAC key; after a host
  restart an old token must fail safely and the renderer refreshes status.
- Preserve non-target entries, array order, target positions, unknown fields,
  existing `onlyReasoning`, and unknown `reasoning` fields. Update only the
  documented connection fields (`url` and the policy-controlled `apiKey`);
  never rebuild or normalize the existing entry. Write backup then primary by
  flush/sync plus
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

| Condition                                                                                                          | Required result                                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Non-Codex app calls a native-feature command                                                                       | Command rejects before TOML analysis or patch.                                                                                                              |
| Complete Codex TOML cannot be parsed                                                                               | Keep both controls visible and disabled; reject capability writes and never reconstruct the document.                                                       |
| Image header has conflicting case variants or an invalid shape                                                     | Show a non-sensitive diagnostic; preserve on unrelated save; explicit image control normalizes, replaces, or deletes only under the documented repair rule. |
| `supports_websockets` has a non-boolean value                                                                      | Show a diagnostic; preserve on unrelated save; explicit enable overwrites with `true`, explicit disable deletes.                                            |
| Chat/Anthropic/official/managed/proxy Provider saves with `supports_websockets = true`                             | Save succeeds. Return model/proxy risk codes when applicable; do not rewrite the choice.                                                                    |
| Fixed official Provider has empty TOML and both controls remain off                                                | Preserve empty TOML and create no Provider table or capability metadata.                                                                                    |
| Persisted session ID is empty, starts with a hyphen, or contains characters outside the conservative ASCII grammar | Keep the session visible but omit `resumeCommand`; never interpolate the raw ID into a shell command.                                                       |
| DB/provider action succeeds but live Codex bytes are unchanged                                                     | Return `liveConfigChanged: false`; do not ask to restart.                                                                                                   |
| Several/non-identical trusted installations or running instances exist                                             | Return ambiguous/unavailable; do not close or launch any process.                                                                                           |
| Graceful exit exceeds 8 seconds                                                                                    | Require the opaque second-confirmation token; no automatic force kill.                                                                                      |
| User cancels a pending Codex force-restart continuation                                                            | Best-effort discard that capability only; do not close, terminate, launch, or disclose any process/installation/token existence.                            |
| New process is absent at 15 seconds or installation drifts                                                         | Return restart failure; retain saved configuration and direct user to manual restart.                                                                       |
| WorkBuddy URL is non-HTTP(S), has credentials/query/fragment, or redirect leaves origin                            | Return `WORKBUDDY_INVALID_URL` or `WORKBUDDY_FETCH_REDIRECT_REJECTED`; do not send credentials onward.                                                      |
| WorkBuddy response exceeds 2 MiB, times out, or has malformed `data[]`                                             | Return bounded fetch error; retain no model IDs from that response.                                                                                         |
| Existing models JSON is invalid/not-array/contains an invalid entry                                                | Return safe config error with only an index when applicable; do not repair or overwrite it.                                                                 |
| Revision changes before a WorkBuddy save or confirmed overwrite                                                    | Return `concurrent_modification`; create no backup and write no primary.                                                                                    |
| Existing WorkBuddy target IDs arrive without a valid matching overwrite token                                      | Return `overwrite_confirmation_required` with a fresh opaque token and unique `existingIds`; create no backup and write no primary.                         |
| WorkBuddy overwrite token is malformed, mismatched, expired, or reused                                             | Return the structured token error; consume it once and create no backup or primary write.                                                                   |
| WorkBuddy UI receives a truncated result                                                                           | Keep the truncation warning visible until a subsequent successful non-truncated fetch replaces it.                                                          |

## 5. Good / Base / Bad Cases

- Good: A Provider named `OpenAI` with a third-party URL, or any official,
  managed OAuth, proxy-taken-over, or ordinary Codex Provider, opens the same
  two controls. Enabling image support canonicalizes only the managed header;
  comments and other valid headers remain, and a live byte change may prompt
  for a trusted restart once.
- Base: A historical provider lacks the marker and managed header. The editor
  displays pending-on, but cancelling the dialog creates no TOML/database
  migration. A later save records either the explicit enabled or disabled
  decision.
- Good: A Chat-format Provider saves WebSocket with a non-GPT model while
  takeover is enabled. The mutation succeeds and returns both warning codes;
  the renderer emits one combined localized warning instead of a success toast.
- Base: A fixed official Provider remains empty while both controls are off,
  creates a minimal ChatGPT-authenticated `custom` table on first enable, and
  removes only an unchanged owned skeleton after the last capability is off.
- Bad: API-format change silently deletes WebSocket, proxy projection forces
  it to false, a save rejects non-Responses format, or a model/proxy warning is
  treated as proof that the upstream transport works.
- Bad: Renderer sends `{ pid: 1234 }`, a different `app`, or a launch path to
  a restart command; a process-name scan kills `codex.exe`; the backend accepts
  any such control.
- Good: WorkBuddy fetches an ordered model response, returns the first 1,000
  unique IDs plus `truncated: true`, and a user resubmits the frozen duplicate
  request with the backend-issued one-time overwrite token against the same
  revision. Non-target JSON, `onlyReasoning`, and extra `reasoning` keys remain.
- Base: The user explicitly allows an empty key; fetch/save sends no
  Authorization and existing per-model keys remain unless clear-existing is
  selected.
- Bad: A generic model fetcher sorts IDs, tries several endpoint suffixes,
  forwards a key to a cross-origin redirect, silently removes invalid JSON
  entries, or deletes the Windows target before rename.

## 6. Tests Required

- TypeScript: test legacy WorkBuddy visibility/order, top-level isolation, all
  four locale key sets, password/default key lifecycle, HTTP warning, persistent
  truncation, duplicate-dialog frozen request/retry, all Codex Provider
  categories showing initially collapsed capability controls, document-vs-field
  diagnostics, format-change preservation, add/update warning-toast
  merge/repetition/failure behavior, and Codex capability/restart dialogs.
- Rust Codex: TOML comments/order/unknown headers, case-insensitive managed
  header repair and invalid-shape preservation, historical marker migration,
  official delayed generation/safe cleanup, WebSocket writes for Responses,
  Chat, and Anthropic without a format gate, invalid-field explicit repair,
  exact-host and malicious authority/path cases for any vendor catalog
  capability grant, GPT/non-GPT/mixed/empty warning matrices, normal/official
  proxy projection and restore, live-byte change truth table, command app guard,
  and fake-platform trusted restart state machine including graceful timeout,
  force confirmation/cancellation, original-installation drift, and 15-second
  verification failure.
- Rust WorkBuddy: URL normalization/rejection, redirection and Authorization
  policy, timeout/2 MiB bounds, malformed entries after cap, exact order and
  case-sensitive de-duplication, no-key behavior, HMAC revision opacity and
  API-Key-only revision conflict, overwrite-token request/revision binding,
  expiry and single consumption, unknown-field/`onlyReasoning` preservation,
  backup/primary failure paths, and test-home isolation. Tests must not access
  the real profile.
- Local gates when dependencies permit:
  - `mise exec -- pnpm typecheck`
  - `mise exec -- pnpm format:check`
  - `mise exec -- pnpm test:unit`
  - `mise exec -- pnpm run build:renderer`
  - `mise exec -- cargo fmt --check --manifest-path src-tauri/Cargo.toml`
  - `mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml --offline --locked -- -D warnings`
  - `mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --offline --locked`
  - `git diff --check`

  Do not characterize these as native E2E, platform, CI, or release evidence.

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
