# Merge CC Switch v3.19.2 — Implementation Plan

1. verify baseline `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`, recovery ref, origin, fetch-only upstream, and clean scoped worktree
2. fetch and verify full v3.19.2 identity
3. audit affected product/workflow/license/data files
4. prepare merge and resolve semantic conflicts
5. create the isolated two-parent merge commit without version/toolchain/workflow modernization
6. record conflict and provenance evidence and run the checks available on the merged baseline

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform scope, unresolved limitations, and the owning spec updates. No parent NO-GO condition may be downgraded; the accepted workflow-only protection risk does not waive any graph, identity, license, data, or validation gate.
