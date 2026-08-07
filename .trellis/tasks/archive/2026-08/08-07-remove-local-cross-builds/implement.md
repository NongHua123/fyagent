# Remove local cross-platform builds — Implementation Plan

1. inventory references after upstream merge
2. move Windows manifest/installer/MSI/signing-layer assertions to native Release contracts
3. delete executable scripts and dedicated cross-build tests
4. clean mise/Rust declarations, targets, `llvm-tools`, and locks
5. remove or rewrite active specs/docs and add negative contract checks
6. run native-host and Release-contract checks

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform scope, unresolved limitations, and the owning spec updates. No parent NO-GO condition may be downgraded; the accepted workflow-only protection risk does not waive any removal or native-release safety gate.
