# FyAgent Modernization Parent PRD

## Objective

Deliver FyAgent `0.3.0` as six ordered implementation workstreams while preserving evidence, Git provenance, FyAgent identity, product behavior, and formal release integrity. The approved execution includes the implementation PR, merge commit to `main`, immutable `v0.3.0` tag, stable unsigned GitHub Release, closeout PR, and final Trellis archive.

## Scope

1. complete explicit merge of CC Switch v3.19.2;
2. remove local Linux/WSL-to-Windows/macOS cross builds;
3. redesign mise/uv/Python and repository task API;
4. modernize Required CI and formal Release;
5. remove the DEP0040 dependency root cause;
6. migrate current documentation and Trellis contracts and supersede old tasks.

## Non-goals

- no Windows or macOS signing, notarization, staple, signing credentials, or signed release mode;
- no branch/tag ruleset, branch protection, or Release environment approval configuration;
- no product runtime API, database schema, data-path, or user-data migration;
- no rewrite of historical design evidence;
- no conversion of optional product-runtime mise compatibility into a hard dependency.

## Cross-workstream invariants

- Upstream merge is an isolated explicit merge commit and happens first.
- FyAgent product identity, license boundary, data paths, features, schema 16, and backups remain; the canonical product version becomes `0.3.0`, never upstream `3.19.2`.
- Formal release assets originate only from GitHub Actions and comprise exactly ten unsigned installers plus separately allowlisted manifest, build metadata, and attestations.
- The public repository restores automatic CI and a safe automatic Labeler.
- Release eligibility is enforced by workflow evidence only: product/tag consistency, `origin/main` ancestry, same-SHA `CI / Required`, and repository/workflow identity. The absence of administrator-enforced rulesets/environments is an accepted, documented residual risk.
- The implementation PR is merged with GitHub's merge-commit strategy so the isolated upstream two-parent merge ancestry remains reachable.
- Local project operations use canonical `mise run` tasks.
- Evidence is labeled observed/decision/proposed/pending verification.
- Any NO-GO condition stops dependent work.

## Acceptance

- [ ] all six child tasks complete in dependency order; Child 3 awaits the
      closeout PR's native Windows ARM64 uv/Python/Trellis smoke;
- [x] current specs and docs match implemented task/workflow behavior;
- [x] automatic PR and post-merge `CI / Required` pass without claiming the branch is administrator-protected;
- [x] unsigned full-matrix preflight proves the full ten-asset, manifest, metadata, and attestation contract;
- [x] `v0.3.0` is published from the exact passing `main` SHA as a stable, non-prerelease GitHub Release and independently reverified;
- [ ] the closeout branch's native gate and aggregate CI pass, after which all
      six children and this parent are archived in dependency order inside the
      same PR before its final CI and merge;
- [ ] risk register has no unresolved NO-GO item; the Child 3 native smoke must
      pass before the Windows ARM64 development-platform claim is closed;
- [x] source, decision, and file traceability are complete for implementation and Release; this branch adds the final closeout record.
