# Modernize CI and Release workflows — Implementation Plan

1. reconcile merged upstream workflows
2. preserve a host-native-only local command surface and reject all local
   cross-OS/cross-architecture compilation, packaging, and verification routes
3. restore safe automatic Labeler and pin explicit runner labels and all third-party Action SHAs
4. build automatic PR/main/merge-group/manual CI matrix and fail-closed `CI / Required` aggregate gate
5. implement strict toolchain/workflow contract checks
6. split platform Release jobs and Linux containers
7. implement workflow-only formal eligibility, exact allowlists, manifest, metadata, mandatory attestations, and final publish gate
8. after repeated full-preflight failures, replace compatibility candidates
   with a schema-owned Windows Installer query module, a native x64/ARM64
   temporary-MSI fixture in Required CI, and a source-explicit runner/container
   metadata writer with direct process tests
9. for every authorized remote run, keep observation in the initiating main
   flow: synchronously wait for the whole run to become `completed`, read the
   final run/job result once, and fetch failed-job logs only after failure
10. only after the shifted-left gates pass in both PR and exact-main CI, run
    unsigned full-matrix preflight; publish only immutable `v0.3.0` from the
    exact passing main SHA and independently reverify it
11. [native gate complete; archive pending] in the closeout PR, extend the native
    Windows x64/ARM64 Required jobs with the locked uv/Python setup and Trellis
    task-list smoke, then archive only after both native legs and the aggregate
    Required gate succeed

## Completion Evidence

Attach exact commands, relevant logs/artifacts, changed-file list, platform
scope, unresolved limitations, and the owning spec updates. Real
PR/main/preflight/formal Release and independent post-download evidence now
exist, and the release gate is GO. PR #8 first failed closed in run
`31264604075`; commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd`
corrected the ARM64 setup-uv request, and run `31265504901` passed x64 job
`93122857985`, ARM64 job `93122858012`, and Required job `93123992476`.
This child stays open only for the remaining closeout operations. The accepted
workflow-only protection risk is documented but does not waive
any Required, asset, manifest, metadata, attestation, or publish gate.

Local validation includes Prettier, focused contract tests, active-only negative
scans, link checks, decision-number uniqueness, Trellis task validation, and
diff checks. It must also record cleanup of any diagnostic non-host processes,
temporary directories, and build outputs. Local Windows Light/MSI diagnostics
are never acceptance. The PR #8 jobs above, rather than local static contracts,
are the native evidence for the expanded Windows smoke. The final design-package
manifest is rebuilt and verified. D114 remains N/A; ordered archives, journal,
final PR CI/merge, exact-main CI, and branch cleanup remain pending.
The full five-target preflight is not a low-level debugging loop: an unknown COM
projection result or undocumented hosted-image variable must return to its
smallest authoritative production/test boundary, not gain a compatibility
fallback or another expensive trial run.
