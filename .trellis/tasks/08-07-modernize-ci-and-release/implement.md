# Modernize CI and Release workflows — Implementation Plan

1. reconcile merged upstream workflows
2. preserve a host-native-only local command surface and reject all local
   cross-OS/cross-architecture compilation, packaging, and verification routes
3. restore safe automatic Labeler and pin explicit runner labels and all third-party Action SHAs
4. build automatic PR/main/merge-group/manual CI matrix and fail-closed `CI / Required` aggregate gate
5. implement strict toolchain/workflow contract checks
6. split platform Release jobs and Linux containers
7. implement workflow-only formal eligibility, exact allowlists, manifest, metadata, mandatory attestations, and final publish gate
8. for every authorized remote run, keep observation in the initiating main
   flow: synchronously wait for the whole run to become `completed`, read the
   final run/job result once, and fetch failed-job logs only after failure
9. run unsigned full-matrix preflight; after PR merge, publish only immutable `v0.3.0` from the exact passing main SHA and independently reverify it

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform scope, unresolved limitations, and the owning spec updates. This child remains open until real PR/main/preflight/Release evidence exists. The accepted workflow-only protection risk is documented but does not waive any Required, asset, manifest, metadata, attestation, or publish gate.

Local validation includes Prettier, focused contract tests, active-only negative
scans, link checks, decision-number uniqueness, Trellis task validation, and
diff checks. It must also record cleanup of any diagnostic non-host processes,
temporary directories, and build outputs. Local Windows Light/MSI diagnostics
are never acceptance. Do not trigger or monitor Actions as part of this local
implementation pass; all remote evidence remains Pending/NO-GO until separately
authorized and completed under the synchronous observation contract.
