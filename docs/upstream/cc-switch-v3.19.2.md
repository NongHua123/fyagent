# CC Switch v3.19.2 Upstream Provenance

This ledger records the source identity and ancestry of the CC Switch v3.19.2
integration. It is not a FyAgent Release Note and does not change FyAgent's
product version from `0.3.0` to the upstream version.

## Verified source and graph

| Field                       | Verified value                                         |
| --------------------------- | ------------------------------------------------------ |
| Authorized FyAgent baseline | `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`             |
| Merge first parent          | `194edb22ef6896f865e08a21b27d5b846dbaf54d`             |
| Upstream repository         | `https://github.com/farion1231/cc-switch.git`          |
| Upstream remote policy      | fetch enabled; push URL `DISABLED`                     |
| Annotated tag               | `v3.19.2`                                              |
| Tag object                  | `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`             |
| Peeled commit               | `43eaf07355af145aebfee301801779e824d4c221`             |
| Merge base                  | `28529620f438b2ed25c812f6364825d846a4a9d6` (`v3.19.1`) |
| FyAgent two-parent merge    | `f4462765e9b3a2efd1deb13aabf3ce349166a058`             |
| Merge second parent         | `43eaf07355af145aebfee301801779e824d4c221`             |
| Integration date            | 2026-08-08 (Asia/Shanghai)                             |

The local annotated tag object and peeled commit matched the upstream remote.
The merge commit has exactly the two parents shown above, with the verified
upstream commit as its second parent, and the upstream tag is its ancestor. The
merge was created with explicit `--no-ff --no-commit` semantics and semantic
conflict resolution; it was not squashed, rebased, or globally resolved to one
side.

## Provenance and license boundary

CC Switch-derived code, history, notices, and attribution retain their MIT
ancestry. The upstream `v3.19.2` Release Note bodies entered the ancestry-only
merge and were removed from the active FyAgent documentation in the later
documentation commit. This ledger and the concise FyAgent CHANGELOG source
entry preserve provenance without presenting upstream release marketing as a
FyAgent release.

FyAgent-owned components and modifications remain under the repository's
published PolyForm Noncommercial terms. See [LICENSE](../../LICENSE),
[LICENSING.md](../../LICENSING.md), and
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). No upstream partner,
sponsorship, affiliate, or tracking metadata became a FyAgent product claim.

## FyAgent contracts preserved through the merge

The merge retained the following FyAgent-specific boundaries while importing
shared upstream correctness, security, compatibility, and performance work:

- product/runtime identity: `FyAgent`, `fyagent`, `com.fyagent.desktop`, and
  `fyagent://`;
- persistence: `~/.fyagent`, `fyagent.db`, `FYAGENT_*`, the FyAgent SQL export
  header, schema **16**, and existing backup behavior;
- Windows protected installation, manifest, single-instance, and activation
  boundaries;
- WorkBuddy and the FyAgent responsive shell;
- mixed licensing, repository identity, and neutral provider behavior without
  upstream promotion/tracking fields.

The isolated merge deliberately left version `0.2.1` in place. The later
toolchain/version commit moved the canonical FyAgent application version to
`0.3.0`, independently of the upstream source version.

## Conflict and validation summary

Thirty-three conflicts were resolved by identity/data/license/security
precedence, then shared upstream behavior, then FyAgent-only behavior. The
review specifically combined upstream read limits, containment and symlink
protection, SQL import hardening and batching, response-body caps, management
search/bulk state, quota behavior, and provider updates with the FyAgent
contracts above.

At the isolated merge boundary, format, type, frontend unit, Rust format/check/
Clippy/test, JSON, conflict-marker, unmerged-index, identity/promotion, schema
16, and Git whitespace checks passed. Those Linux-host checks did not claim
Windows, macOS, ARM, installer, CI, or public Release evidence; each belongs to
its later native/remote gate.

The long-term engineering contract is
[CC Switch Upstream Synchronization](../../.trellis/spec/backend/upstream-sync.md).
