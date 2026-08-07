# Modernize CI and Release workflows — Implementation Plan

1. reconcile merged upstream workflows
2. restore safe automatic Labeler and pin explicit runner labels and all third-party Action SHAs
3. build automatic PR/main/merge-group/manual CI matrix and fail-closed `CI / Required` aggregate gate
4. implement strict toolchain/workflow contract checks
5. split platform Release jobs and Linux containers
6. implement workflow-only formal eligibility, exact allowlists, manifest, metadata, mandatory attestations, and final publish gate
7. run unsigned full-matrix preflight; after PR merge, publish only immutable `v0.3.0` from the exact passing main SHA and independently reverify it

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform scope, unresolved limitations, and the owning spec updates. This child remains open until real PR/main/preflight/Release evidence exists. The accepted workflow-only protection risk is documented but does not waive any Required, asset, manifest, metadata, attestation, or publish gate.
