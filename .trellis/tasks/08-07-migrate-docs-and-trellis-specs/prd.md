# Migrate documentation and Trellis contracts — PRD

## Goal

Make current human/agent documentation, Trellis specs/tasks, and generated task references match the implemented repository contracts while preserving historical evidence.

## Scope

- README x4, CONTRIBUTING, PR template, Flatpak/visual docs
- workflow and operational skills
- rewrite/add/remove long-term specs and indexes
- generate task reference and documentation checks
- remove upstream v3.19.2 release-note files and add provenance ledger
- add CHANGELOG plus English/Chinese/Japanese `v0.3.0` Release Notes with prominent unsigned-install guidance
- synchronize the full v1-0.3.0 design package, decision/risk/traceability/file matrices, validation evidence, and checksum manifest
- archive the five old active tasks as superseded; archive the new children/parent as completed only after their real local/remote/release gates pass

## Constraints

- do not rewrite historical design bodies
- do not bulk-fork trellis-meta references
- do not claim proposed behavior is implemented before verification
- archive old tasks with reason/parent replacement metadata
- do not claim administrator branch/tag protection or Release-environment approval; document the accepted workflow-only residual risk
- do not provide scripts that disable Gatekeeper/SmartScreen or strip quarantine

## Acceptance Criteria

- [ ] active docs use canonical `mise run` tasks and contain no retired or noncanonical direct project commands
- [ ] spec indexes and generated task docs are consistent
- [ ] old task archives preserve content and say superseded
- [ ] source/provenance and FyAgent CHANGELOG records remain
- [ ] documentation contract checks pass
- [ ] Release Notes and download guidance explain unsigned macOS Privacy & Security → Open Anyway and Windows SmartScreen behavior safely
- [ ] real CI/preflight/Release/asset/digest/attestation evidence is recorded before final new-task archival

## Evidence Boundary

Implementation was authorized on 2026-08-08. Authorization is not completion evidence: current-contract migration, old-task archive, formal Release closeout, new-task archive, and journal evidence remain pending until each stage is real.
