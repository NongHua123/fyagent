# CC Switch v3.19.2 merge evidence

## Source and graph identity

| Evidence                   | Verified value                                         |
| -------------------------- | ------------------------------------------------------ |
| Authorized source baseline | `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`             |
| Recovery ref               | `refs/backup/fyagent-v0.3.0-baseline`                  |
| Origin                     | `https://github.com/NongHua123/fyagent.git`            |
| Upstream fetch             | `https://github.com/farion1231/cc-switch.git`          |
| Upstream push              | `DISABLED`                                             |
| Tag object                 | `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`             |
| Peeled commit              | `43eaf07355af145aebfee301801779e824d4c221`             |
| Merge base                 | `28529620f438b2ed25c812f6364825d846a4a9d6` (`v3.19.1`) |
| Two-parent merge           | `f4462765e9b3a2efd1deb13aabf3ce349166a058`             |
| First parent               | `194edb22ef6896f865e08a21b27d5b846dbaf54d`             |
| Second parent              | `43eaf07355af145aebfee301801779e824d4c221`             |

The local tag object and peeled commit matched `git ls-remote` for the upstream
repository. `git merge-base --is-ancestor refs/tags/v3.19.2 f4462765` passed,
and the merge commit contains exactly two parents in the order above.

## Conflict ledger

The merge produced 33 conflicts. Each was resolved semantically; no global
`ours` or `theirs` operation was used.

### Repository identity and public text

- `.github/FUNDING.yml`: removed both the stale FyAgent sponsor anchor and the
  upstream account sponsor mapping; FyAgent currently publishes no repository
  sponsorship link.
- `CONTRIBUTING.md` and four README files: retained the FyAgent public-product
  and mixed-license baselines for this isolated merge. Broader documentation
  reconciliation belongs to the documentation child.
- Three DeepSeek routing guides: absorbed upstream v3.19.1+ behavior while
  distinguishing the upstream feature version from the FyAgent product
  version and keeping current-product wording as FyAgent.

### Package, runtime identity, and persistence

- `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and
  `src-tauri/tauri.conf.json`: retained FyAgent package/Tauri identity and
  `0.2.1` for the pure merge; preserved upstream's non-Windows direct `libc`
  dependency and regenerated the one-edge lock change mechanically.
- `codex_config.rs`: combined the 32 MiB read cap, containment, canonicalization,
  and symlink-escape protection with FyAgent catalog names and paths.
- `database/backup.rs`: combined SQL authorizer, atomic import, legacy export
  compatibility, batching, and performance tests with `FYAGENT_TEST_HOME`,
  `~/.fyagent/fyagent.db`, and the FyAgent export header.
- `proxy/handlers.rs`: retained `fyagent_proxy_error` and mapped the new 128 MiB
  response-body limit failure.

### Renderer behavior and tests

- `App.tsx`, `AppSwitcher.tsx`, and `subscription.ts`: retained WorkBuddy and
  the FyAgent responsive shell while integrating management search/bulk state,
  navigation/interaction blocking, Skills discovery/check updates, and
  per-account OAuth quota.
- Deep-link and App integration tests retain both branches' behavior and use
  `fyagent-last-view`.
- Eight provider preset files and four locales retain neutral provider
  functionality but remove `partnerPromotion`, `partnerPromotionKey`,
  `isPartner`, and tracking metadata. A duplicate Qiniu Grok Build preset found
  by the full unit suite was removed while preserving the upstream ordering.

### Automatically merged identity hazards

- Replaced `CC_SWITCH_TEST_HOME` with an RAII-restored
  `FYAGENT_TEST_HOME` in the ignored real-corpus replay test so a failure cannot
  touch the real FyAgent database or leak environment state.
- Replaced current-product `CC Switch` names in OMO errors, model-fetch/catalog
  comments, and OAuth quota comments with FyAgent.
- Preserved former-name negative fixtures, upstream issue/repository URLs, and
  license/provenance facts as reviewed exceptions.

## Validation

- `pnpm run version:check`: passed at the isolated merge version `0.2.1`.
- `pnpm format:check`: passed.
- `pnpm typecheck`: passed.
- `pnpm test:unit`: 129 files and 884 tests passed.
- `cargo fmt --all --check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo check --workspace --all-targets --locked --offline --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --workspace --all-targets --locked --offline --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed.
- `cargo test --workspace --locked --offline --manifest-path src-tauri/Cargo.toml`: passed.
- JSON parsing, conflict-marker scan, unmerged-index scan, identity/promotion
  scan, schema `16`, and `git diff --check`: passed.

The ignored real Codex corpus replay was not run because it requires an
explicit local corpus and opt-in environment. Windows/macOS/ARM packaging and
formal Release evidence remain owned by later children and are not inferred
from this Linux-host merge validation.
