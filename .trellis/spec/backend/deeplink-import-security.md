# Deep-Link Import Security Contract

## 1. Scope / Trigger

This contract applies whenever a `fyagent://v1/import` payload crosses the
custom-protocol parser, renderer confirmation UI, Tauri commands, or provider
configuration service. It is required because `DeepLinkImportRequest` is an
untrusted cross-layer DTO that can carry credentials and request a change to a
live provider configuration.

The protocol may request an outcome; it must never be treated as evidence that
the user approved the outcome. In particular, a provider link with
`enabled=true` is a request to activate, not authority to switch the current
provider.

## 2. Signatures

The wire shape is camelCase in TypeScript and Rust serde output:

```ts
type DeepLinkImportRequest = {
  version: "v1";
  resource: "provider" | "prompt" | "mcp" | "skill";
  enabled?: boolean;
  // Written only by the renderer confirmation UI.
  activationApproved?: boolean;
  // Resource-specific fields, including endpoint, apiKey, config, and content.
};
```

```rust
#[tauri::command]
fn parse_deeplink(url: String) -> Result<DeepLinkImportRequest, String>;

#[tauri::command]
fn merge_deeplink_config(
    request: DeepLinkImportRequest,
) -> Result<DeepLinkImportRequest, String>;

#[tauri::command]
async fn import_from_deeplink_unified(
    state: State<'_, AppState>,
    request: DeepLinkImportRequest,
) -> Result<serde_json::Value, String>;

ProviderService::add_draft(
    state: &AppState,
    app_type: AppType,
    provider: Provider,
) -> Result<bool, AppError>;

ProviderService::switch(
    state: &AppState,
    app_type: AppType,
    id: &str,
) -> Result<SwitchResult, AppError>;
```

`DeepLinkImportRequest.activation_approved` serializes as
`activationApproved`. It is optional, is valid only for the `provider`
resource, and is meaningful only together with `enabled == Some(true)`.

## 3. Contracts

- `parse_deeplink_url` accepts the bounded `fyagent://v1/import` envelope and
  **always** returns `activation_approved: None`; an `activationApproved`
  query parameter is never a protocol capability.
- Every IPC command that receives a `DeepLinkImportRequest` calls
  `validate_deeplink_request` before merging or importing. Direct renderer IPC
  must receive the same envelope, control-character, double-percent-encoding,
  resource, and activation-field validation as a protocol invocation.
- The renderer resets its local approval state to `false` for every received
  link. Only the dedicated, initially unchecked provider-activation checkbox
  may set `activationApproved: true`; it sends `false` otherwise.
- A provider import first stores the provider through `add_draft`. It calls
  `ProviderService::switch` only when both `enabled == Some(true)` and
  `activation_approved == Some(true)`. Without that conjunction, an import
  may create/update the draft record but must not select it or write the live
  provider configuration.
- Configuration merging is asynchronous in the renderer. Each received link
  has a monotonically increasing sequence; an older merge completion or import
  completion must not replace, close, or inherit approval from the latest
  visible confirmation.
- Parser, merge, and import failures are renderer-safe generic strings. They
  must not include the source URL, API key, nested configuration, or raw parser
  error. The renderer must likewise ignore a `deeplink-error` event payload.
- Any prompt content that the confirmation can write is rendered completely in
  a bounded, scrollable review region. Do not hide a writable tail behind a
  preview truncation.

## 4. Validation & Error Matrix

| Condition                                                                                                                             | Required result                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| URL has a wrong scheme/version/action, duplicate parameter, control character, second percent encoding, or exceeds a documented bound | Parse is rejected with a generic parse error; no credential or URL is returned/logged to the renderer. |
| Renderer IPC sends an oversized or otherwise invalid DTO                                                                              | `merge_deeplink_config` and import commands reject it with the generic operation error.                |
| `activationApproved` is present for `prompt`, `mcp`, or `skill`                                                                       | Reject the request.                                                                                    |
| `activationApproved=true` but `enabled` is absent or false                                                                            | Reject the request.                                                                                    |
| Provider link requests `enabled=true`, but the checkbox remains unchecked                                                             | Store only a draft; leave current provider and live configuration unchanged.                           |
| Provider link requests `enabled=true`, and the user explicitly checks approval                                                        | Store the draft and then switch to that exact imported provider.                                       |
| Older config-merge/import promise completes after a newer link is shown                                                               | Ignore its UI state transition.                                                                        |
| Deep-link error event includes a URL or credentials in an older host payload                                                          | Show only the translated generic error; do not inspect, log, or interpolate payload fields.            |

## 5. Good / Base / Bad Cases

- Good: A `provider&enabled=true` link shows the intended target, warning, and
  unchecked activation box. Clicking ordinary Import saves a draft; checking
  the box and clicking Import and activate switches only the reviewed draft.
- Base: A provider link without `enabled=true` imports as a draft without
  offering activation. Prompt, MCP, and skill imports retain their resource
  semantics and cannot carry an activation approval bit.
- Bad: Parsing `activationApproved=true` from the URL, preserving the previous
  dialog's checked state for the next link, or calling `switch` directly after
  `add_draft` merely because `enabled=true` was supplied.

## 6. Tests Required

- Rust parser/unit tests must assert that every parser constructor sets
  `activation_approved` to `None`, and that direct DTO validation rejects
  invalid activation combinations without leaking secrets.
- Provider-service tests must assert that `add_draft` preserves an existing
  current provider and its live configuration, while the explicit approved
  path is the only path that calls `switch`.
- `tests/components/DeepLinkImportDialog.test.tsx` must cover: full writable
  prompt review, unchecked and checked provider submissions, ignored error
  payloads, stale merge completion, and stale import completion.
- Run the declared fake/static checks through mise: `pnpm test:unit`,
  `pnpm typecheck`, and `pnpm format:check`. No real custom-protocol launch or
  desktop application operation is a substitute for these assertions.

## 7. Wrong vs Correct

### Wrong

```rust
if request.enabled == Some(true) {
    ProviderService::switch(state, app_type, &provider_id)?;
}
```

This lets a URL select the active provider and write its live configuration.

### Correct

```rust
ProviderService::add_draft(state, app_type.clone(), provider)?;
if request.enabled == Some(true) && request.activation_approved == Some(true) {
    ProviderService::switch(state, app_type, &provider_id)?;
}
```

The second condition is set only by the current in-app confirmation UI after
the user has reviewed the complete import payload.
