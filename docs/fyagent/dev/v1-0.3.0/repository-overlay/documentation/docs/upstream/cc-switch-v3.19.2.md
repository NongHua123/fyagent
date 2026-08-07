# CC Switch v3.19.2 Upstream Provenance

> Status: proposed provenance record; the merge has not been executed in this
> documentation-only deliverable.

| Field | Value |
| --- | --- |
| FyAgent source baseline | `55173d2b`, supplied by the project owner |
| FyAgent branch | `feature/fyagent-v1` |
| Upstream repository | `https://github.com/farion1231/cc-switch.git` |
| Upstream formal tag | `v3.19.2` |
| Public short SHA | `43eaf07` |
| Public full commit SHA | `43eaf07355af145aebfee301801779e824d4c221` |
| Upstream release date | 2026-08-06 |
| Upstream license | MIT |
| FyAgent merge commit | pending implementation |

The uploaded source archive does not contain `.git`; this record does not claim
that tag ancestry or the merge already exists. Before implementation:

```bash
git fetch upstream tag v3.19.2
git rev-parse v3.19.2^{commit}
git show --no-patch --format='%H %D' v3.19.2
```

Verify that the fetched tag resolves to `43eaf07355af145aebfee301801779e824d4c221`, then record the FyAgent merge commit. The merge must
preserve upstream ancestry and MIT notices. Upstream v3.19.2 release-note files
are accepted into the merge commit and removed in a later FyAgent documentation
commit; this concise record and the FyAgent CHANGELOG retain provenance without
rebranding or redistributing the complete upstream release notes.

FyAgent's optional product-runtime compatibility for CLI tools installed through
mise follows upstream CC Switch behavior. This is not a hard dependency for
installing or starting FyAgent and is distinct from mise's repository-development
role.
