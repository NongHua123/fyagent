# Eliminate DEP0040 punycode root cause — Implementation Plan

1. reproduce and capture current chain after upstream merge
2. remove import/dependency
3. regenerate lock with canonical pnpm version
4. add success, non-2xx, empty-response, cross-realm Fetch/MSW probes and native-global preconditions
5. add ordinary throw, focused pending+throw, reverse-dependency, and suppression checks
6. update quality/upstream difference docs

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform scope, unresolved limitations, and the owning spec updates. No parent NO-GO condition may be downgraded; the accepted workflow-only protection risk does not waive any dependency, behavior, deprecation, or warning gate.
