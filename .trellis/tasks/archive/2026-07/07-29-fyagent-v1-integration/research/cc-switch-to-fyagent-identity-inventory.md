# Research: CC Switch to FyAgent identity inventory

- Query: Inventory remaining `CC Switch` / `cc-switch` / `cc_switch` / `ccswitch` identities, classify them by runtime surface, and identify the migration gates for a complete FyAgent rename without running a local build.
- Scope: internal
- Date: 2026-07-30

## Subsequent decision

After this inventory was written, the user explicitly chose the clean-break
option. The compatibility recommendation below is therefore research context,
not the implementation contract: FyAgent does not migrate, alias, discover,
import, or clean up the former application's data, deep links, autostart entry,
or serialized identity markers. The repository name and real repository URLs,
historical and license attribution, upstream references, and exact external
partner/referral tokens remain factual exceptions and must not be fabricated or
cosmetically replaced.

## Findings

### Executive conclusion

This is not a cosmetic rename. The repository currently uses the old identity as
an application-install identity, persistence namespace, wire-format owner marker,
external referral code, and historical compatibility key. A blind global replace
would make existing data appear lost, split Windows/macOS application identity,
break old deep links and cloud-sync roots, and prevent old Codex projections or
conversation payloads from being cleaned up or replayed.

The active task's reviewed contract currently says the opposite of the new user
request: it explicitly preserves identifier, deep-link scheme, data directories,
internal package names, and `LICENSE` (`.trellis/tasks/07-29-fyagent-v1-integration/prd.md:33-38`,
`.trellis/tasks/07-29-fyagent-v1-integration/design.md:10-13`). Before implementation,
the main session must update the task requirements/design/plan. The new requirement
cannot be treated as a continuation of the old branding-only acceptance criterion.

There are two distinct acceptance meanings which cannot both be achieved literally:

1. **Zero old identity literals anywhere**: clean break; existing installations,
   data, links, exports, sync roots, and serialized payloads are not compatible.
2. **Complete active FyAgent identity with safe migration**: all new writes and
   primary identifiers use FyAgent, but narrowly allowlisted legacy literals remain
   in migration readers/cleanup paths, historical fixtures, legal history, partner
   referral codes, and upstream citations.

The second is the safe recommendation. A final `rg` audit must distinguish active
identity from deliberate legacy compatibility; otherwise a "zero match" assertion
would force removal of the very code needed to preserve user data.

### Recommended identity map

The reverse-DNS identifier and repository URLs require owner confirmation. The
following is the internally consistent default mapping, not evidence that the
project owns a `fyagent.com` DNS namespace.

| Old active identity | Suggested new identity | Compatibility rule |
| --- | --- | --- |
| `CC Switch`, `CC-Switch` | `FyAgent` | New display text only; historical release text may retain the historical name. |
| npm/package `cc-switch` | `fyagent` | Rename `package.json`; lockfile only if it materializes the root package name. |
| Cargo package / default bin `cc-switch` | `fyagent` | Changes the default executable to `fyagent`/`fyagent.exe`; synchronize CI and Flatpak. |
| Rust lib `cc_switch_lib` | `fyagent_lib` | Mechanical source/import rename; no persisted compatibility burden. |
| Tauri identifier `com.ccswitch.desktop` | proposed `com.fyagent.desktop` | High-risk OS identity change; confirm reverse-DNS choice and installer upgrade strategy first. |
| URI scheme `ccswitch://` | `fyagent://` | Register and generate new scheme; safe migration requires accepting the old scheme as an alias for at least one transition line. |
| `~/.cc-switch/` | `~/.fyagent/` | One-time, idempotent, conflict-aware migration before DB/log/settings initialization. |
| `cc-switch.db` / `cc-switch.log` | `fyagent.db` / `fyagent.log` | Rename together with the config directory; all direct consumers must converge on one resolver. |
| `CC Switch` Windows Run value | `FyAgent` | Remove/migrate the old Run value before or while enabling the new value to avoid duplicate startup. |
| tray ID `cc-switch` | `fyagent` | Internal runtime rename plus assertion update. |
| `CC_SWITCH_TEST_HOME` | `FYAGENT_TEST_HOME` | Test-only; rename all test guards together. |
| `CC_SWITCH_GDK_BACKEND` | `FYAGENT_GDK_BACKEND` | Public launch override; temporarily read new first, legacy second if compatibility is required. |
| `CC_SWITCH_DEVICE_NAME` | `FYAGENT_DEVICE_NAME` | Public sync/device override; same dual-read/deprecation rule. |
| serialized `cc_switch` skill location | `fyagent` | New serializer value plus legacy deserialize alias and TS normalization. |
| localStorage keys `cc-switch-*` / `cc-switch:*` | `fyagent-*` / `fyagent:*` | One-time read-through migration preserves theme, last view/app, and snippets. |
| cloud root `cc-switch-sync` | `fyagent-sync` | Do not silently abandon existing WebDAV/S3 data; migrate persisted settings or provide legacy discovery/import. |
| Codex provider `cc-switch-official` | `fyagent-official` | New writes use FyAgent; restore/cleanup must recognize and remove the old provider table. |
| Codex catalog `cc-switch-model-catalog.json` | `fyagent-model-catalog.json` | Migrate config pointer/file; ownership cleanup must recognize both filenames. |
| SQL header `-- CC Switch SQLite 导出` | `-- FyAgent SQLite 导出` | New exports write FyAgent; importer must continue accepting the old trusted header. |
| payload prefixes `ccswitch-openai-reasoning-v1:` and `ccswitch-anthropic-thinking-v1:` | `fyagent-...-v1:` or a versioned `v2` prefix | Encoder writes new; decoder must accept old or old sessions lose replay semantics. |
| proxy markers/error codes `cc_switch_*`, `[cc-switch: ...]` | `fyagent_*`, `[fyagent: ...]` | Treat error codes as a public local API and tool/reasoning markers as serialized wire data; dual-parse where replayed. |
| Flatpak ID/files `com.ccswitch.desktop*` | proposed `com.fyagent.desktop*` | Rename manifest, desktop, AppStream, icon, launchable and install paths atomically. |
| GitHub `farion1231/cc-switch`, `ccswitch.io`, `cc-switch-website` | current FyAgent repository/site | New canonical targets were not established by repository source; do not invent them. Preserve upstream citations separately. |

### 1. User-visible brand and partner text

Files found:

- `README_ZH.md`, `README_JA.md`, `README_DE.md`: still start with `CC Switch`,
  include `ccswitch.io`, old repository badges/downloads, old package/install names,
  `ccswitch://`, and `~/.cc-switch` paths. `README.md` has a FyAgent heading but
  deliberately still contains old referral codes/URLs and old runtime paths
  (`README.md:20-181`, `README.md:246`, `README.md:296-300`, `README.md:358-378`).
- `CONTRIBUTING.md`, `SECURITY.md`, `SUPPORT.md`: old public project name and
  upstream issue/discussion/security/release routes (`CONTRIBUTING.md:1-13`,
  `SECURITY.md:5-22`, `SUPPORT.md:7-27`). These are current public surfaces,
  unlike immutable release history.
- `.github/ISSUE_TEMPLATE/*.yml`, `.github/FUNDING.yml`, `.github/CODEOWNERS`:
  old issue labels/examples, repository routes, funding route, and repository
  description (`.github/ISSUE_TEMPLATE/bug_report.yml:13-23`,
  `.github/ISSUE_TEMPLATE/question.yml:60-62`).
- `flatpak/com.ccswitch.desktop.metainfo.xml:4-24` and
  `flatpak/com.ccswitch.desktop.desktop:3-6`: visible Flatpak name, description,
  homepage/bugtracker and launcher identity.
- `src/i18n/locales/{en,ja,zh,zh-TW}.json`: visible legacy paths/log names/cloud
  roots and partner promo codes at corresponding lines 293, 406, 524, 589, 598,
  708, 756, and the partner-description block around 981-1042. The translation
  key `ccSwitch` already renders `FyAgent`, but the key itself is still internal
  legacy naming.
- `src/config/{claude,claudeDesktop,codex,gemini,grokBuild,hermes,opencode,openclaw}ProviderPresets.ts`:
  externally assigned referral query values and codes such as `aff=cc-switch`,
  `CCSWITCH`, `CC-SWITCH`, `utm_content=ccswitch`, and `utm_source=cc_switch`.
- `src/main.tsx:51`, `src/hooks/useDirectorySettings.ts:66`,
  `src/components/UsageScriptModal.tsx:78-97,1567`, and
  `src/hooks/useImportExport.ts:147`: paths, example User-Agent and export
  filename that a user can see or copy.

Migration/risk:

- Brand prose and examples can be changed directly.
- Referral/coupon codes and tracking parameters are external contracts, not
  free-form branding. Mechanical replacement can invalidate discounts or
  attribution. Preserve them until each partner supplies a FyAgent code; brand
  the surrounding prose as FyAgent and document the legacy code as a coupon.
- A new canonical repository and support/security route is required before old
  operational links can be changed. Upstream citations are handled in category 7.

Static verification:

- Search current public/UI/config surfaces for old spellings, then allowlist
  only partner code values and migration/help text that intentionally names a
  legacy path.
- Verify all four locale files have identical identity-bearing key coverage.
- Verify partner URLs are unchanged unless a partner-approved replacement is
  supplied; do not infer a new referral code from the product name.

### 2. Tauri identifier, deep link, app data/config/log/database and wire namespaces

Files found:

- `src-tauri/tauri.conf.json:5,57-60`: `com.ccswitch.desktop` and the `ccswitch`
  deep-link scheme.
- `src-tauri/Info.plist:5-13`: explicit macOS `ccswitch` URL registration.
- `src-tauri/src/lib.rs:251-262,1044-1049,1092,1831-1840`: scheme validation,
  Linux handler path under the old identifier, and runtime URL routing.
- `src-tauri/src/deeplink/{mod,parser,mcp,prompt,provider,skill,tests}.rs` and
  `src-tauri/tests/deeplink_import.rs`: parser contract and fixtures for
  `ccswitch://`; `deplink.html` is a standalone generator/demo with the old
  scheme throughout (for example lines 351, 864, 1466, 1530-1531).
- `src-tauri/src/config.rs:16-23,182-203`: authoritative home override and
  `~/.cc-switch/cc-switch.db`, including a Windows v3.10.3 alternate-`HOME`
  recovery path. This code already documents that a path change looks like
  data loss.
- `src-tauri/src/lib.rs:451-521`, `src-tauri/src/database/mod.rs:99-101`,
  `src-tauri/src/database/backup.rs:301`, `src-tauri/src/panic_hook.rs:3-28`,
  `src-tauri/src/settings.rs:339,562`, and `src-tauri/src/services/env_manager.rs:71`:
  log, database, crash, settings and backup consumers. Several bypass the
  central config-dir resolver and must be fixed together.
- `src-tauri/src/app_config.rs:587-588,664`, `src/main.tsx:51`,
  `src/hooks/useDirectorySettings.ts:66`, four locales, README/user manuals:
  user-visible legacy path and backup guidance.
- `src/App.tsx:124,144`, `src/components/AppSwitcher.tsx:31`,
  `src/main.tsx:93,116`, `src/components/theme-provider.tsx:30`,
  `src/components/sessions/SessionManagerPage.tsx:79-81`, and the three
  `src/components/providers/forms/hooks/use*CommonConfig*.ts` files: browser
  storage namespaces.
- `src-tauri/src/settings.rs:101`, `src/components/settings/WebdavSyncSection.tsx`
  (default/fallbacks at lines 169, 270, 287, 343, 383, 464, 680, 944-945,
  1086, 1369), and `src-tauri/src/services/{s3_sync,webdav_sync}.rs`: persisted
  WebDAV/S3 remote namespace `cc-switch-sync`.
- `src-tauri/src/services/skill.rs:38-44`, `src/types.ts:240`,
  `src/lib/schemas/settings.ts:39`, `src/lib/api/skills.ts:211`, and settings UI:
  serialized `cc_switch` enum value.
- `src-tauri/src/codex_config.rs:16-22,189,1020-1168,1512-1681`: Codex takeover
  provider key and generated catalog filename. The cleanup/ownership logic
  compares exact old names.
- `src-tauri/src/codex_history_migration.rs:47-51,768-788` and its fixtures:
  `ccswitch` is an intentional legacy model-provider identifier used to recover
  existing history; it is not the current display brand.
- `src-tauri/src/database/backup.rs:17-18,168-170,397`: the SQL export trust
  header intentionally keeps old exports importable.
- `src-tauri/src/proxy/providers/reasoning_bridge.rs:11` and
  `transform_codex_anthropic.rs:28`: replay-sensitive payload prefixes.
- `src-tauri/src/proxy/tool_media.rs:14-16,101,253`,
  `src-tauri/src/proxy/providers/transform_responses.rs:24`, and transformer
  consumers: tool-result markers embedded into request/response content.
- `src-tauri/src/proxy/handlers.rs:1881-1899`: `cc_switch_*` public error codes.
- `src-tauri/src/proxy/providers/xai_oauth_auth.rs:22`,
  `src-tauri/src/services/subscription_grok.rs:569`, and
  `src-tauri/src/commands/misc.rs:971`: outbound User-Agent identities.
- `src-tauri/src/main.rs:32-33` and
  `src-tauri/src/services/sync_protocol.rs:351`: public environment variables;
  test files use `CC_SWITCH_TEST_HOME` broadly.
- `src-tauri/src/tray.rs:151,1117`: internal tray identity and assertion.

Migration/risk gates:

1. Run directory/file migration before any DB, settings, log or crash-hook open.
   Migration must be idempotent and must not overwrite if both old and new
   directories exist. A both-exist case needs an explicit conflict policy or a
   fail-closed support error; silently merging SQLite/config files is unsafe.
2. Preserve the current Windows real-profile versus injected-`HOME` fallback
   while locating a legacy directory. After successful migration, all writers
   must use the new central resolver.
3. New SQL exports/payloads/catalogs may use FyAgent, but old readers/cleanup
   aliases are required. Removing old prefix/header/provider recognition makes
   existing user content unreadable or leaves stale takeover config behind.
4. A new deep-link scheme cannot open existing links. If zero old literals is
   mandatory, this is a documented breaking change. Otherwise register/parse
   both and generate only `fyagent://`.
5. Changing the cloud default creates a separate remote tree. Existing explicit
   `remoteRoot` values remain old automatically; users relying on defaults need
   a settings migration or legacy-root discovery.

Static verification:

- Add source-level migration fixtures for old-only, new-only, neither, and
  both-exist directory states; source review can verify they contain no overwrite
  path even when local tests are deferred to CI.
- Audit direct `.cc-switch`/`cc-switch.db` joins and require production writes
  to flow through one new resolver; legacy literals should occur only in the
  migration module/tests/help text.
- Enumerate every encoder/decoder pair for SQL headers, Codex provider/catalog,
  reasoning prefixes, deep links, serialized enum and cloud root. Assert in
  source that new writes use FyAgent and old reads remain accepted where chosen.

### 3. Rust crate/lib/bin and npm/package names

Files found:

- `package.json:2`: root npm package `cc-switch`.
- `src-tauri/Cargo.toml:2,13`: Cargo package/default binary `cc-switch` and
  library `cc_switch_lib`; `src-tauri/Cargo.lock:760` mirrors the package name.
- `src-tauri/src/main.rs:7,40`: library entry calls.
- Every Rust integration-test crate imports `cc_switch_lib`:
  `src-tauri/tests/{app_config_load,app_type_parse,deeplink_import,hermes_roundtrip,import_export_sync,mcp_commands,profile_roundtrip,provider_commands,provider_service,proxy_commands,skill_sync,support}.rs`.
- `.github/workflows/release.yml:474-478` locates `cc-switch.exe`; Flatpak
  `command`/`Exec` also expects `cc-switch`.

Migration/risk:

- With no explicit `[[bin]]`, the Cargo package rename changes the executable
  path. Cargo manifest, lock entry, lib name, main imports, integration tests,
  release portable lookup and Flatpak command must land in one change.
- Renaming only `package.json` is independent of the Rust binary but should be
  reflected in any package-manager metadata if present.

Static verification:

- `rg -n "cc_switch_lib|target/.*/cc-switch\\.exe|command: cc-switch|Exec=cc-switch"`
  should have no active matches after the coordinated rename.
- Cross-check the Cargo package/lib names against `main.rs`, all integration
  imports, release binary lookup and Flatpak executable declarations.

### 4. Flatpak/Linux desktop/AppStream/DBus identities and filenames

Files found:

- `flatpak/com.ccswitch.desktop.yml:1,7,78-98`: Flatpak app ID, command, module,
  source filenames, installed desktop/AppStream filenames and icon ID.
- `flatpak/com.ccswitch.desktop.desktop:3-6`: visible name, `Exec`, icon ID.
- `flatpak/com.ccswitch.desktop.metainfo.xml:3-24`: AppStream ID/name/description,
  desktop launchable, binary, homepage and bug tracker.
- `flatpak/README.md:3,30-65`: old manifest/deb/bundle filenames, app ID,
  run command and old config permission example.
- `.gitignore:22`: `flatpak/cc-switch.deb`.
- `src-tauri/src/lib.rs:1044-1049`: expected Linux deep-link handler path under
  `~/.local/share/com.ccswitch.desktop/applications/cc-switch-handler.desktop`.

No separate explicit application-owned DBus well-known name/service file was
found. `libdbusmenu` references in the Flatpak manifest are a dependency, not
the product identity. The Flatpak ID nevertheless serves as the desktop and
AppStream namespace and commonly becomes the sandbox/application identity.

Migration/risk:

- Rename the three `flatpak/com.ccswitch.desktop*` files and all internal IDs,
  installed destinations and source references atomically. A new Flatpak ID is
  a distinct application for users; it does not transparently upgrade the old
  installation or inherit its sandbox data/permissions.
- Coordinate the Cargo binary rename before changing `Exec`/`command`.

Static verification:

- Assert manifest `id`, desktop filename, desktop `Icon`, AppStream `id`,
  `launchable`, icon install target and `flatpak run` value are identical.
- Assert `command`, desktop `Exec`, AppStream `binary` and Cargo bin are identical.
- Verify no source reference points to a renamed-away Flatpak file.

### 5. Windows/macOS install, upgrade and auto-start identity

Files found:

- `src-tauri/tauri.conf.json:3-5`: product is already `FyAgent` but bundle ID is
  still `com.ccswitch.desktop`.
- `src-tauri/wix/per-user-main.wxs`: the generated template consumes
  `{{upgrade_code}}`, stores product registry state under
  `Software\\{{manufacturer}}\\{{product_name}}`, and writes
  `{{bundle_id}}` as the shortcut `System.AppUserModel.ID`; it also registers
  each configured deep-link protocol.
- `src-tauri/src/auto_launch.rs:18-27`: intentionally keeps Windows Run value
  name `CC Switch` so FyAgent can control an old startup entry. macOS uses the
  application bundle path.
- `.github/workflows/release.yml:259-354,463-503`: public artifacts are already
  FyAgent, but portable Windows binary lookup is still `cc-switch.exe`.
- `src-tauri/Info.plist:5-13`: old macOS URL scheme; the Tauri bundle identifier
  is supplied by `tauri.conf.json`.

Migration/risk gates:

- Changing `identifier` changes Windows AppUserModelID and macOS bundle identity.
  It can produce a separate OS identity, reset per-app permissions/associations,
  and affect in-place upgrade behavior. The repository does not pin an explicit
  WiX upgrade code in Tauri config; the template only receives one. If an
  in-place MSI upgrade is required, capture/freeze the prior UpgradeCode rather
  than assuming a changed identifier derives the same value. CI-generated WiX
  evidence is required because no local build is authorized.
- Product name is already FyAgent, so the current WiX registry/install folder
  is product-name based. Older CC Switch installations may have a different
  product registry key/install directory; the current source has no explicit
  migration search for that old install key.
- Replace the auto-start value only with cleanup/migration of the old value;
  otherwise startup can become duplicated or the old entry becomes unmanaged.
- A new macOS bundle ID/scheme may coexist with the old LaunchServices/login-item
  identity. Real upgrade, signing, TCC, URL-open and login-item behavior remain
  CI/native acceptance, not statically proven.

Static verification:

- Inspect CI-produced WiX for the intended UpgradeCode, executable name,
  AppUserModelID, product registry path and protocol keys; do not claim upgrade
  compatibility from `tauri.conf.json` alone.
- Source-audit that enabling/disabling startup handles the selected migration
  policy for both Run value names.
- Verify macOS config/Info.plist agree on the new scheme and bundle identity.

### 6. GitHub Actions, release and repository URLs

Files found:

- `.github/workflows/release.yml:259-354,463-539,599-612`: release artifact and
  release-note names are FyAgent; only the old Rust executable lookup remains at
  lines 474-478.
- `.github/workflows/claude.yml:56`: public automation prompt still names the
  repository `cc-switch`.
- `scripts/generate-download-manifest.mjs:3-4`: comments bind the output to
  `ccswitch.io/download` and `cc-switch-website`.
- `.github/FUNDING.yml`, `.github/ISSUE_TEMPLATE/*.yml`, `CONTRIBUTING.md`,
  `SECURITY.md`, `SUPPORT.md`, Flatpak metadata and translated README files route
  operational users to `farion1231/cc-switch`.
- `src-tauri/src/proxy/providers/transform_codex_chat.rs:4114` and three
  `docs/guides/codex-claude-routing-guide-*.md:129` entries are historical
  issue/PR citations, not current support routing.

Migration/risk:

- The repository does not establish a replacement website/repository URL.
  Operational links must point to the actual PR/release/security owner; inventing
  a `fyagent` URL would create broken or unsafe reporting routes.
- Preserve upstream issue/PR links as attribution/citation even after canonical
  project routes move. Add an explicit upstream attribution section if needed.
- Renaming the Rust bin requires release lookup changes in the same commit or
  Windows portable packaging fails in CI.

Static verification:

- Separate URL searches into `operational route` versus `historical upstream
  citation`; require no old operational routes, but allowlist exact citations.
- Parse/review release paths so every produced artifact uses FyAgent and every
  binary lookup uses the renamed executable. CI, not local compilation, provides
  packaging evidence.

### 7. Historical docs, license, upstream attribution and Trellis artifacts

Files found:

- `LICENSE`: MIT copyright for Jason Young; it contains no CC Switch string.
  It is legal provenance and must not be rewritten merely to satisfy branding.
- `CHANGELOG.md` and `docs/release-notes/**`: extensive historical release names,
  paths and repository references.
- `docs/user-manual/{en,ja,zh}/**` and `docs/guides/**`: current instructions
  mixed with historical/upstream citations; current install/data/deep-link
  instructions need migration, citations need preservation.
- `docs/fyagent/dev/v1/**`: deliberately describes the old compatibility
  boundary and contains old identifiers in requirements, examples and audit
  commands.
- `.trellis/spec/backend/application-brand-assets.md:15-24,40-42` and
  `.trellis/spec/frontend/index.md:1-3`: current specs explicitly preserve and/or
  describe the old identities.
- `.trellis/tasks/07-29-fyagent-v1/{prd,implement}.md` and
  `.trellis/tasks/07-29-fyagent-v1-integration/{prd,implement}.md`: task history
  and current acceptance evidence contain old literals. Current task artifacts
  conflict with the new requirement; completed/historical artifacts should not
  be mechanically falsified.

Required exceptions:

1. Preserve `LICENSE`, copyright and required notices exactly unless the legal
   owner explicitly authorizes a legal change.
2. Preserve accurate old release names in changelog/release-note history. A
   short lineage note is safer than rewriting history.
3. Preserve exact upstream issue/PR links and contributor attribution.
4. Preserve externally assigned partner coupon/referral values until replaced
   by the partner.
5. Preserve narrowly scoped legacy literals in migration readers/cleanup tests
   if backward compatibility is selected.
6. Update the **current** Trellis PRD/design/implement/spec contracts to the new
   identity decision; do not rewrite old task evidence as though prior releases
   had always been FyAgent.

Static verification:

- Maintain an explicit allowlist grouped by legal history, upstream citations,
  partner codes and migration fixtures. Every other old-identity match fails the
  audit.
- Confirm `LICENSE` bytes and upstream attribution links remain intact.
- Confirm current manuals/specs no longer instruct new users to use old active
  paths/schemes/packages except in a clearly labeled migration section.

### 8. Tests and negative assertions

Files found:

- `tests/releaseWorkflow.test.ts:22-54` already asserts FyAgent release naming
  and rejects `CC Switch`, `CC-Switch`, and `ccswitch.io` in the release workflow,
  but it does not reject the old executable path.
- `src-tauri/src/tray.rs:1116-1118` freezes `TRAY_ID = "cc-switch"`.
- `src-tauri/tests/support.rs:10-32` and most Rust integration tests freeze
  `CC_SWITCH_TEST_HOME`, `.cc-switch`, `cc-switch.db`, `cc_switch_lib`, deep links,
  skill paths and SQL export names.
- `src-tauri/src/codex_config.rs` and
  `src-tauri/src/codex_history_migration.rs` have extensive fixtures for old
  provider/catalog identities; these must be split into new-write assertions and
  intentional legacy-read assertions rather than globally replaced.
- `tests/hooks/useDirectorySettings.test.tsx:188,218`, settings/sync component
  tests, and import/export tests freeze old app paths/cloud roots/export headers.
- Transformer tests freeze old reasoning/tool marker prefixes and error codes.

Required test shape for CI (local execution intentionally deferred):

- New installation writes only FyAgent directory/database/log/settings names.
- Old-only data migrates once without loss; repeated startup is idempotent;
  both-old-and-new conflicts fail closed or follow a reviewed deterministic rule.
- Old SQL exports, Codex provider/catalog projections, reasoning prefixes,
  serialized skill location and deep links remain readable only if compatibility
  is selected; all new writes/links use FyAgent.
- Windows startup migration removes/controls the old Run value and creates the
  new one without duplication.
- Release test rejects `cc-switch.exe` and requires `fyagent.exe` after the Cargo
  rename.
- Flatpak consistency test validates ID/filename/icon/launchable/bin agreement.
- Repository-wide negative assertion permits old literals only in the explicit
  allowlist. Do not assert a raw zero-match while compatibility/history remains.

### Proposed static-only validation checklist

No compilation, build, `cargo check/test`, `pnpm build`, or local bundling is
part of this research. After implementation, the main session can perform these
non-compiling checks before pushing the draft PR; CI owns compile/test/package
evidence.

1. `git diff --check` and scoped diff review for accidental historical/legal or
   partner-code rewrites.
2. Parse edited JSON with PowerShell `ConvertFrom-Json`; review TOML/YAML/XML
   syntax with repository-available non-building parsers only if already present.
3. Run four separate `rg` audits:
   - active source/config/package/CI identifiers (must be FyAgent),
   - migration literals (exact allowlist only),
   - legal/history/upstream citations (exact allowlist only),
   - partner referral/coupon values (unchanged unless approved replacements exist).
4. Cross-reference Cargo package/lib/bin against `main.rs`, all Rust test imports,
   `.github/workflows/release.yml`, Flatpak `command`/`Exec`/`binary`, and ignore
   rules.
5. Cross-reference Tauri identifier/deep-link against Info.plist, runtime parser,
   standalone deep-link generator, manuals, Flatpak ID policy, WiX template inputs,
   and negative tests.
6. Cross-reference the app-data resolver against every database, log, crash,
   settings, backup, skills, directory-picker and user-facing path consumer.
7. Verify no generated `dist/`, `node_modules/`, or `src-tauri/target/` artifact is
   manually committed as the source rename. They were excluded from the source
   inventory and will be regenerated by CI/build environments.

## External references

None. This inventory is based only on the current repository. Tauri/WiX behavior
that depends on generated installer metadata must be confirmed by CI artifact
inspection rather than asserted from memory.

## Related specs

- `.trellis/spec/backend/application-brand-assets.md` — currently says icon
  changes preserve identifiers, deep links, data directories and internal names;
  this must be revised if the new identity migration is accepted.
- `.trellis/spec/backend/github-release-workflow.md` — owns FyAgent release asset
  naming and branch/tag release boundaries.
- `.trellis/spec/backend/codex-desktop-installer.md` — installer behavior remains
  relevant, but it does not define the host application's identity migration.
- `.trellis/spec/frontend/index.md` and
  `.trellis/spec/frontend/state-management.md` — frontend persistence and public
  text guidance; localStorage migration should follow the existing state boundary.
- `.trellis/spec/guides/cross-layer-thinking-guide.md` — the rename crosses
  config, persistence, installer, renderer, CI and external protocol boundaries.

## Caveats / Not Found

- The replacement canonical GitHub repository, website, support/security routes,
  reverse-DNS owner and Windows manufacturer identity were not specified in the
  repository evidence. They are decision inputs, not safe guesses.
- No explicit app-owned DBus service/name was found; Flatpak/desktop/AppStream IDs
  are the Linux identity surfaces found.
- The prior WiX `UpgradeCode` is not pinned in source. Static source alone cannot
  prove whether a changed Tauri identifier upgrades or side-by-side installs;
  inspect CI-generated WiX/MSI metadata.
- `dist/`, `node_modules/`, `src-tauri/target/`, `.git/`, and binary ZIP artifacts
  were excluded from the source inventory. They can contain stale generated text
  but should not be hand-edited.
- Ordinary historical prose was not exhaustively enumerated line-by-line after
  the main session requested that research stop expanding. The identity-bearing
  runtime/config/CI surfaces and historical document families above were covered.
- No local compile, test, renderer build, bundle build, installation, signing,
  notarization, native launch, or visual validation was run.
