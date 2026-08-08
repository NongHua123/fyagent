# Modernize CI and Release workflows — Design

## Architecture

Use a static contract layer, platform check layer, and release transaction. Keep publish isolated behind all build/sign/verify jobs. Resolve Linux runner retirement by separating host runner from pinned older same-architecture user space.

## Failure Policy

The task is fail-closed for its owned contracts. A missing required source identity, strict check, platform proof, lock consistency, signing result, or documentation contract is not silently downgraded to success.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
