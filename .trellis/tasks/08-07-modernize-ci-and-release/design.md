# Modernize CI and Release workflows — Design

## Architecture

Use a static contract layer, platform check layer, safe automatic Labeler, and unsigned release transaction. Keep publish isolated behind eligibility, native build, structure, exact-asset, manifest, metadata, and attestation jobs. Resolve Linux runner retirement by separating explicit Ubuntu 24.04 host runners from digest-pinned Ubuntu 22.04 same-architecture user space.

Formal source eligibility is workflow-enforced rather than administrator-enforced. It verifies the immutable SHA against `origin/main`, product/tag `0.3.0`, same-SHA `CI / Required`, and expected repository/workflow identity. This is intentionally weaker than branch/tag rulesets and a protected environment and must be documented as an accepted residual supply-chain risk.

## Failure Policy

The task is fail-closed for runner/tool identity, action pins, minimal permissions, Required dependency results, repository/source eligibility, five native target groups, exact ten installers, manifest, metadata, attestations, and one-time publish. ARM preview runner unavailability may be retried but never replaced by cross-build or partial release.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
