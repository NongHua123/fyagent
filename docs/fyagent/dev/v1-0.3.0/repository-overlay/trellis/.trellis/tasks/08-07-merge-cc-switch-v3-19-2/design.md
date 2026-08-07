# Merge CC Switch v3.19.2 — Design

## Architecture

Use the existing fork/upstream Git model. `upstream:check/fetch/audit/merge:prepare` automate only mechanical safety. Conflict decisions are documented per file. Non-conflicting upstream files enter the merge commit, including release notes; the later docs task removes the upstream release-note files.

## Failure Policy

The task is fail-closed for its owned contracts. A missing required source identity, strict check, platform proof, lock consistency, signing result, or documentation contract is not silently downgraded to success.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
