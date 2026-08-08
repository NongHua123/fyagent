# Remove local cross-platform builds — Design

## Architecture

Treat cross-build removal as a dependency graph, not folder deletion. Remove executable paths first, then task/tool declarations, tests, current specs/docs, and stale output references. Preserve archived task/design evidence and Git history.

## Failure Policy

The task is fail-closed for its owned contracts. A missing required source identity, strict check, platform proof, lock consistency, signing result, or documentation contract is not silently downgraded to success.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
