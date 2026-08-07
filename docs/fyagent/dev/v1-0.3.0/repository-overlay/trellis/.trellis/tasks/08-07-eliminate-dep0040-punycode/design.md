# Eliminate DEP0040 punycode root cause — Design

## Architecture

Delete the unnecessary compatibility layer because Node 24 is the strict project baseline. Validate behavior rather than existence only. Distinguish legitimate newer `whatwg-url`/`tr46` dependencies by version and reverse origin.

## Failure Policy

The task is fail-closed for its owned contracts. A missing required source identity, strict check, platform proof, lock consistency, signing result, or documentation contract is not silently downgraded to success.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
