# Merge CC Switch v3.19.2 — Design

## Architecture

Use the existing fork/upstream Git model and verify tag object `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a` plus peeled commit `43eaf07355af145aebfee301801779e824d4c221`. Because the canonical upstream task API is created later, this child uses explicit read-only Git inspection followed by `git merge --no-ff --no-commit`. Conflict decisions are documented per file and resolved by identity, shared upstream logic, engineering governance, and FyAgent-only capability layers. Non-conflicting upstream files enter the merge commit, including release notes; the later docs task removes their product-facing bodies while retaining provenance.

## Failure Policy

The task is fail-closed on tag/remotes, two-parent ancestry, FyAgent branding/bundle/deep-link/data/database/schema/environment/license identity, and conflict resolution. Modernization changes are not allowed in the merge commit.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
