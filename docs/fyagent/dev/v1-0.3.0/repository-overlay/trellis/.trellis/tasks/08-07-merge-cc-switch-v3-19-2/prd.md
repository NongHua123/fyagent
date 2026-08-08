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
- no automatic commit or push
- no separate upstream-product acceptance workflow

## Acceptance Criteria

- [ ] tag is ancestor of merge result
- [ ] merge commit has both parent histories
- [ ] FyAgent identity/data/licensing/version checks pass
- [ ] ordinary Required CI is green after PR integration

## Evidence Boundary

This task begins in `planning`. Nothing in this artifact claims the merge, configuration, tests, CI, or Release has already been completed. Pending platform/Git evidence must be attached during implementation.
