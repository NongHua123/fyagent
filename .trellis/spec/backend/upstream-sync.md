# CC Switch Upstream Synchronization Contract

## 1. Scope / Trigger

Read this contract before fetching, auditing, merging, or documenting a CC
Switch upstream release. It governs Git remote identity, immutable tag
verification, merge ancestry, semantic conflict resolution, FyAgent identity,
licensing provenance, and the boundary between an upstream merge and later
FyAgent modernization commits.

FyAgent product versions are independent from CC Switch versions. The current
product target is `0.3.0`; merging CC Switch `v3.19.2` must not set any FyAgent
package or artifact version to `3.19.2`.

## 2. Signatures

The verified integration topology is:

```text
origin fetch/push = https://github.com/NongHua123/fyagent.git
upstream fetch    = https://github.com/farion1231/cc-switch.git
upstream push     = DISABLED
```

The v0.3.0 synchronization anchor is:

| Field                     | Verified value                             |
| ------------------------- | ------------------------------------------ |
| FyAgent source baseline   | `55173d2b32c4acf182b6ec504d7ad326ade2bb9b` |
| Upstream tag object       | `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a` |
| Upstream peeled commit    | `43eaf07355af145aebfee301801779e824d4c221` |
| Merge base                | `28529620f438b2ed25c812f6364825d846a4a9d6` |
| Reviewed two-parent merge | `f4462765e9b3a2efd1deb13aabf3ce349166a058` |

The canonical mechanical sequence is:

```bash
git remote get-url origin
git remote get-url upstream
git remote get-url --push upstream
git ls-remote --tags upstream refs/tags/v3.19.2 'refs/tags/v3.19.2^{}'
git fetch --no-tags upstream refs/tags/v3.19.2:refs/tags/v3.19.2
git merge-base <fyagent-head> refs/tags/v3.19.2
git merge --no-ff --no-commit refs/tags/v3.19.2
```

Future `mise run upstream:*` wrappers must enforce the same inputs and failure
conditions. `upstream:merge:prepare` may stop only in an uncommitted merge
state; it must not resolve conflicts, create a commit, tag, or push.

## 3. Contracts

### Remote and immutable-source boundary

- `origin` is the only normal push target. `upstream` must have no usable push
  URL, and automation must never repair or replace that disabled push setting.
- Audit a stable annotated tag by both tag-object SHA and peeled commit SHA.
  A matching tag name or short SHA is insufficient.
- Never merge a mutable upstream branch when an approved immutable tag exists.
  Emergency commit picks require a separate recorded decision.
- Create a read-only recovery ref before the first integration mutation. Do not
  use destructive reset or overwrite unrelated working-tree changes.

### Merge and conflict boundary

- The upstream integration commit is an explicit two-parent merge. Do not
  squash, rebase, or reconstruct the upstream history.
- The merge commit contains upstream files and only the conflict resolutions
  needed to make that upstream state a valid FyAgent state. Product version
  changes, local build retirement, mise/uv, CI/Release, dependency cleanup,
  and documentation modernization belong to later commits.
- Resolve conflicts in this order:
  1. preserve FyAgent identity, data, licensing, and security boundaries;
  2. follow shared upstream correctness, compatibility, security, and
     performance behavior;
  3. leave FyAgent engineering-governance modernization to its owning task;
  4. retain FyAgent-only product behavior unless an explicit decision removes
     it.

- Never apply repository-wide `ours` or `theirs`. Record semantic conflicts
  and test the combined result.

### FyAgent invariants

- Preserve `FyAgent`, `fyagent`, `com.fyagent.desktop`, `fyagent://`,
  `~/.fyagent`, `fyagent.db`, `FYAGENT_*`, the FyAgent SQL export header, and
  schema version `16`.
- Preserve the mixed licensing model. CC Switch-derived code and attribution
  remain MIT provenance; FyAgent-owned code remains under the repository's
  stated licensing boundary.
- Do not import sponsorship, partner flags, affiliate/tracking parameters, or
  upstream distribution-channel claims. They are neither product behavior nor
  required provenance.
- Product-runtime discovery for external CLI tools managed by mise remains
  optional compatibility. FyAgent installation, startup, and core behavior
  must not require mise.
- Upstream release-note files may enter the ancestry-preserving merge commit.
  A later documentation commit removes their product-facing bodies and records
  concise provenance; never rebrand them as FyAgent release notes.

### Maintained Native Fetch delta

FyAgent removes upstream's test-only `cross-fetch` dependency and
`cross-fetch/polyfill` import after the reviewed v3.19.2 merge. This is an
intentional, independently revertible downstream delta, not a conflict
resolution inside the two-parent merge. It relies on the exact Node version in
`.node-version`, which must provide unmarked native `fetch`, `Headers`,
`Request`, and `Response` globals.

The delta remains valid only while the Native Fetch → MSW → Tauri mock
behavior suite covers JSON success, non-2xx text errors, empty responses, and
cross-realm jsdom Headers, and while the dependency report proves that
`cross-fetch → node-fetch@2 → whatwg-url@5 → tr46@0.0.3 → built-in punycode`
is absent. Modern jsdom dependencies on `whatwg-url@14`, `tr46@5`, and the
userland `punycode@2` package are not the DEP0040 root cause and remain allowed
when both the pnpm lock and `pnpm why --json` explain their reverse paths.

Node 24.19.0's pending-deprecation probe does not reliably surface every warning
originating under dependencies, so it supplements rather than replaces the
lock and reverse-graph checks. Re-evaluate and preferably remove this delta
when a future reviewed upstream release removes the compatibility dependency;
if upstream adopts another Fetch layer, retain FyAgent's native-only boundary
unless its real behavior and deprecation evidence justify a new decision.

## 4. Validation & Error Matrix

| Condition                                                                            | Required result                                                                       |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| `upstream` has a usable push URL                                                     | Stop before fetch/merge; restore the reviewed disabled-push configuration explicitly. |
| Tag object or peeled commit differs from the approved full SHA                       | Stop as a source-identity failure; do not merge a similarly named tag.                |
| Unrecognized dirty files overlap the merge                                           | Stop and preserve them; do not stash, reset, or overwrite them implicitly.            |
| Merge result changes FyAgent to the upstream product/version/data root               | Reject the resolution and restore the FyAgent invariant before testing.               |
| Upstream introduces a real security/correctness fix behind an identity conflict      | Port the shared behavior under FyAgent names and add or retain its regression tests.  |
| Conflict markers or unmerged index entries remain                                    | Do not commit.                                                                        |
| Merge commit is not two-parent or the verified tag is not its ancestor               | Reject the integration; do not proceed to modernization.                              |
| Schema exceeds `16` or user data paths change without an approved migration          | NO-GO; require a separate data decision and migration plan.                           |
| Partner, sponsorship, affiliate, or tracking metadata enters active product surfaces | Remove it and rerun promotion-boundary tests.                                         |
| Only local/static tests exist for platform artifacts                                 | Report platform release evidence as pending; do not infer native acceptance.          |

## 5. Good / Base / Bad Cases

- Good: the reviewed tag is verified by full object identities, merged in a
  two-parent commit, shared fixes work under FyAgent identity, schema remains
  `16`, and later modernization remains independently reviewable.
- Base: an upstream file contains historical `CC Switch` issue URLs or license
  attribution. Preserve those factual references while keeping current-product
  UI, runtime errors, paths, and comments FyAgent-specific.
- Bad: accept a short SHA, enable upstream pushes, choose one side of every
  conflict globally, rename FyAgent to `3.19.2`, copy partner promotion, or
  squash the merge into a linear implementation commit.

## 6. Tests Required

- Verify remote fetch/push URLs, local tag object, peeled commit, merge base,
  parent count, parent order, and `git merge-base --is-ancestor`.
- Run conflict-marker and unmerged-index scans before committing.
- Run the application-identity audit and promotion-boundary tests; classify
  historical/source/negative-fixture exceptions explicitly.
- Run version consistency, JSON/TOML parsing, renderer format/type/unit tests,
  Rust fmt/check/clippy/tests, and the security tests affected by conflict
  resolution.
- Run the DEP0040 JSON report and focused pending-deprecation Native
  Fetch/MSW/Tauri behavior probe while the maintained downstream Fetch delta
  exists.
- Assert schema `16`, FyAgent test-home isolation, database/export-header
  behavior, proxy error mapping, package identity, and Tauri identity.
- Record native/platform/Release evidence separately; a successful Linux host
  merge check does not prove Windows, macOS, ARM, or formal release artifacts.

## 7. Wrong vs Correct

### Wrong

```bash
# A mutable branch, globally chosen conflict side, and squash erase the audit
# boundary and can silently replace FyAgent identity.
git merge upstream/main
git checkout --theirs .
git commit
git rebase -i main
```

### Correct

```bash
git ls-remote --tags upstream refs/tags/v3.19.2 'refs/tags/v3.19.2^{}'
git fetch --no-tags upstream refs/tags/v3.19.2:refs/tags/v3.19.2
git merge --no-ff --no-commit refs/tags/v3.19.2
# Resolve and test every semantic conflict, then create one two-parent commit.
git commit -m "merge(upstream): merge CC Switch v3.19.2"
git merge-base --is-ancestor refs/tags/v3.19.2 HEAD
```
