# Eliminate DEP0040 punycode root cause — Implementation Plan

1. reproduce and capture current chain after upstream merge
2. remove import/dependency
3. regenerate lock with canonical pnpm version
4. add Fetch/MSW and deprecation probes
5. add reverse-dependency and suppression checks
6. update quality/upstream difference docs

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform scope, unresolved limitations, and the owning spec updates. Run the parent GO/NO-GO evaluation before closing.
