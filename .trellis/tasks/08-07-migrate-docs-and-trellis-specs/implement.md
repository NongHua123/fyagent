# Migrate documentation and Trellis contracts — Implementation Plan

1. [done] apply final task/workflow names to docs
2. [done] replace Trellis operational entry points
3. [done] rewrite/add/delete specs and indexes
4. [done] remove upstream release-note bodies, record provenance/CHANGELOG, and add multilingual v0.3.0 Release Notes with safe unsigned guidance
5. [done] update the complete design package and archive exactly five old active tasks as `superseded`
6. [done] generate task docs and run strict local drift/contract scans
7. [native evidence and final manifest complete; archive pending] after Release, land
   closeout evidence and pass the closeout PR's final native Windows
   uv/Python/Trellis gate. PR #8 run `31265504901` passed x64 job `93122857985`,
   ARM64 job `93122858012`, and Required job `93123992476` after fix commit
   `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd`. The final checksum manifest is
   rebuilt and verified; archive this child inside that PR. The parent is archived after
   all children, the journal is recorded after the archive commit, and only
   after final PR/main CI and merge are writable non-main branches removed.

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform
scope, unresolved limitations, and the owning spec updates. Formal Release
evidence is complete and GO. PR #8's native closeout smoke is also complete;
its first run `31264604075` failed closed on the ARM64 setup-uv architecture
selection, and the corrected run `31265504901` passed. The final checksum
manifest is rebuilt and verified; this child remains open only for ordered
archive. D114 remains N/A. Journal, final PR CI/merge, exact-main CI, and branch
cleanup remain later parent-level closeout stages and are not circular
prerequisites for archiving this child.
