# Migrate documentation and Trellis contracts — Design

## Architecture

Use current specs as long-term contracts, task artifacts as implementation evidence, and concise multilingual README onboarding linked to one generated task catalog. Exclude archives and generic template references from current-command scans.

## Failure Policy

The task is fail-closed for its owned contracts. A missing required source identity, strict check, platform proof, lock consistency, signing result, or documentation contract is not silently downgraded to success.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
