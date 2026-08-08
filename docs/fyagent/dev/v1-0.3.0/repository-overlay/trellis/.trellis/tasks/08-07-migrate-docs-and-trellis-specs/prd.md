# Migrate documentation and Trellis contracts — PRD

## Goal

Make current human/agent documentation, Trellis specs/tasks, and generated task references match the implemented repository contracts while preserving historical evidence.

## Scope

- README x4, CONTRIBUTING, PR template, Flatpak/visual docs
- workflow and operational skills
- rewrite/add/remove long-term specs and indexes
- generate task reference and documentation checks
- remove upstream v3.19.2 release-note files and add provenance ledger
- archive all current tasks as superseded

## Constraints

- do not rewrite historical design bodies
- do not bulk-fork trellis-meta references
- do not claim proposed behavior is implemented before verification
- archive old tasks with reason/parent replacement metadata

## Acceptance Criteria

- [ ] active docs have no direct project tool commands or retired tasks
- [ ] spec indexes and generated task docs are consistent
- [ ] old task archives preserve content and say superseded
- [ ] source/provenance and FyAgent CHANGELOG records remain
- [ ] documentation contract checks pass

## Evidence Boundary

This task begins in `planning`. Nothing in this artifact claims the merge, configuration, tests, CI, or Release has already been completed. Pending platform/Git evidence must be attached during implementation.
