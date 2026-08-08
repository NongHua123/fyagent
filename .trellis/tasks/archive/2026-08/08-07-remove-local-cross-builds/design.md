# Remove local cross-platform builds — Design

## Architecture

Treat cross-build removal as a dependency graph, not folder deletion. Extract the Windows release security contract before deleting its script, then remove executable paths, task/tool declarations, targets, `llvm-tools`, dedicated tests, the active cross-build spec, current docs, and stale output references. Preserve archived task/design evidence and Git history. The surviving standard tasks expose only the current host; GitHub Actions remain authoritative for Windows x64/ARM64, Linux x64/ARM64, and macOS Universal builds.

## Failure Policy

The task is fail-closed if any live local cross-build or automatic trust path remains, if native-host development regresses, or if any Windows installer security assertion is lost.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.
