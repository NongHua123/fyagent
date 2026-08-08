# Migrate documentation and Trellis contracts — Implementation Plan

1. [done] apply final task/workflow names to docs
2. [done] replace Trellis operational entry points
3. [done] rewrite/add/delete specs and indexes
4. [done] remove upstream release-note bodies, record provenance/CHANGELOG, and add multilingual v0.3.0 Release Notes with safe unsigned guidance
5. [done] update the complete design package and archive exactly five old active tasks as `superseded`
6. [done] generate task docs and run strict local drift/contract scans
7. [in progress] after Release, land closeout evidence and pass the initial
   closeout PR's final native Windows uv/Python/Trellis gate; then refresh the
   final checksum manifest and archive this child inside that PR. The parent is
   archived after all children, the journal is recorded after the archive
   commit, and only after final PR/main CI and merge are writable non-main
   branches removed.

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform
scope, unresolved limitations, and the owning spec updates. Formal Release
evidence is complete and GO, but this child remains open through closeout. The
accepted workflow-only protection risk does not waive the pending native
closeout smoke, documentation, checksum, or this child's archive gate. Journal,
final CI/merge, and branch cleanup remain later parent-level closeout stages and
are not circular prerequisites for archiving this child.
