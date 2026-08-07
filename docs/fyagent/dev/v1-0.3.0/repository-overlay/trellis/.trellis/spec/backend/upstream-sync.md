# CC Switch Upstream Synchronization Contract

## Remote identity

```text
origin fetch/push = https://github.com/NongHua123/fyagent.git
upstream fetch    = https://github.com/farion1231/cc-switch.git
upstream push     = DISABLED
```

`origin` is the only normal push target. Automation must never restore an upstream push URL or force-push either remote.

## Synchronization unit

Routine synchronization uses a verified stable CC Switch tag and records the full 40-character commit SHA. `main` may be researched but is not the reproducible merge identity. Emergency commit picks require a separate recorded justification.

For the current modernization, the target is a complete merge of `v3.19.2` before all other changes. The user-provided FyAgent base is `55173d2b`; the uploaded archive lacks `.git`, so ancestry and the upstream full SHA remain implementation-time verification gates.

## Merge procedure

```text
1. verify remotes and clean worktree
2. fetch the exact upstream tag
3. verify full tag commit identity
4. audit merge base, commit set, diff, licensing, workflow, dependency, and data files
5. git merge --no-ff --no-commit <verified-tag>
6. resolve semantic conflicts manually
7. create one explicit merge commit
8. apply FyAgent modernization in later commits
```

No global `ours`/`theirs`, squash, rebase, automatic conflict resolution, commit, tag, or push is permitted.

## Conflict precedence

1. Preserve FyAgent identity: brand, repository URLs, bundle ID, deep-link scheme, application data/database/log paths, `FYAGENT_*` environment names, assets, licensing, and independent `0.2.x` line.
2. Default-follow shared upstream product logic: security, data correctness, provider/CLI/MCP/Skills compatibility, platform fixes, proxy correctness, and performance/resource boundaries.
3. Apply the confirmed FyAgent engineering decisions for local cross-build removal, toolchains, mise/uv, CI/release, and DEP0040 in separate follow-up commits.
4. Preserve FyAgent-specific features unless a documented product decision says otherwise.

Every semantic conflict records the upstream behavior, FyAgent behavior, final resolution, rationale, test coverage, and reevaluation condition.

## Product-runtime mise compatibility

Upstream-compatible optional mise CLI discovery/lifecycle support remains. It does not make mise a packaged runtime prerequisite. A machine without mise must install and start FyAgent normally; a user whose external CLI is managed by mise retains upstream-compatible discovery and source-aware behavior.

## Licensing and provenance

CC Switch-derived code remains MIT-licensed and preserves attribution. FyAgent-owned work remains in its repository licensing boundary. The upstream tag, full SHA, merge commit, and source register are retained. Upstream v3.19.2 release-note files are removed in the later documentation task, not rewritten as FyAgent release notes.

## Tasks and tests

`upstream:check`, `upstream:fetch`, `upstream:audit`, `upstream:merge:prepare`, and `upstream:merge:abort` enforce the mechanical boundary. Normal Required CI is the merge gate; no separate upstream-product acceptance workflow is required. Formal multi-platform release behavior remains governed by the Release workflow contract.
