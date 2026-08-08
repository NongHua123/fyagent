# Modernize CI and Release workflows — Design

## Architecture

Use a static contract layer, platform check layer, safe automatic Labeler, and unsigned release transaction. Keep publish isolated behind eligibility, native build, structure, exact-asset, manifest, metadata, and attestation jobs. Resolve Linux runner retirement by separating explicit Ubuntu 24.04 host runners from digest-pinned Ubuntu 22.04 same-architecture user space.

The local execution boundary is orthogonal to the workflow matrix. Standard
local development, build, test, package, and verification commands use only
the current host OS/architecture; they expose no cross target and cannot bridge
to another OS, emulator, copied toolchain, or staged foreign artifact. Every
non-host gate runs on the matching native Actions runner.

Formal source eligibility is workflow-enforced rather than administrator-enforced. It verifies the immutable SHA against `origin/main`, product/tag `0.3.0`, same-SHA `CI / Required`, and expected repository/workflow identity. This is intentionally weaker than branch/tag rulesets and a protected environment and must be documented as an accepted residual supply-chain risk.

## Failure Policy

The task is fail-closed for runner/tool identity, action pins, minimal permissions, Required dependency results, repository/source eligibility, five native target groups, exact ten installers, manifest, metadata, attestations, and one-time publish. ARM preview runner unavailability may be retried only as a separately authorized whole run and is never replaced by cross-build or partial release.

After repeated full-preflight failures, the full Release matrix is not a low-level
debugging loop. Windows Installer query behavior is exercised by a generated
temporary MSI fixture on native Windows x64 and ARM64 in Required CI. Platform
metadata is produced by a directly tested, exact-key, source-explicit writer:
requested runner routing, documented runner context, configured OCI evidence,
and in-container observations remain distinct. Compatibility fallbacks for an
unknown COM projection failure or undocumented hosted-image variables are not
accepted. A new full preflight is allowed only after the implementation PR and
exact-main Required CI both pass these shifted-left gates.

An authorized Actions trigger remains owned by the initiating primary flow.
That flow performs one blocking whole-run wait until `completed`, then reads the
final run/job result once. It neither delegates a background/asynchronous
monitor nor repeatedly polls status; a failed final result alone permits one
failed-job log retrieval. Trigger, observation, retry, tag, and publish remain
separate authority gates.

## Rollback

Keep the work in a reviewable child commit/series. Revert only this child when possible; document any dependency on the isolated upstream merge or earlier child output.

If a non-host local path or detached monitoring path is discovered, stop it,
clean only its explicitly resolved processes/temporary/build outputs, and
return the affected acceptance item to Pending. Diagnostic output from that
path cannot be promoted into release evidence.
