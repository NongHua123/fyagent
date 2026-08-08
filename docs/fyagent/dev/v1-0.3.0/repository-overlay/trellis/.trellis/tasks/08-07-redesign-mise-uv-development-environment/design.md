# Redesign mise and uv development environment — Design

## Architecture

Separate standard declarations, task orchestration, and complex implementation scripts. mise owns tool selection and tasks; uv exclusively owns Python. One rule engine provides human/JSON strict environment evidence. Mutation and interactive tasks are explicit.

## Failure Policy

The task is fail-closed for its owned contracts. A missing required source identity, strict check, platform proof, lock consistency, signing result, or documentation contract is not silently downgraded to success.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
