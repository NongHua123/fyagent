# Merge CC Switch v3.19.2 — PRD

## Goal

Merge the verified CC Switch v3.19.2 tag completely with explicit ancestry while preserving FyAgent identity and recording every semantic conflict.

## Scope

- verify origin/upstream and full tag SHA
- audit merge base and diff
- enter `--no-ff --no-commit` merge
- resolve conflicts by the agreed precedence
- preserve MIT provenance and FyAgent licensing boundary
- run ordinary repository checks available at that stage

## Constraints

- no squash/rebase/cherry-pick substitute
- no global ours/theirs
- no modernization mixed into the merge commit
- no push to `upstream`; create the reviewed merge commit only after conflict and identity checks
- no separate upstream-product acceptance workflow

## Acceptance Criteria

- [ ] tag is ancestor of merge result
- [ ] merge commit has both parent histories
- [ ] FyAgent identity/data/licensing/version checks pass
- [ ] the stage-local identity, licensing, schema/data, and available repository checks pass; final automatic CI remains a parent/CI-child gate

## Evidence Boundary

Implementation was authorized on 2026-08-08. Authorization is not completion evidence: every checklist item remains pending until the real graph, conflict record, identity checks, and stage-local validation are attached.
