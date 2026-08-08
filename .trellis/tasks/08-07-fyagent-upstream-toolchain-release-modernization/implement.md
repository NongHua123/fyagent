# Parent Implementation Plan

1. Verify the exact baseline, remotes, upstream tag object/peeled SHA, recovery ref, integration branch, and public-repository state.
2. Complete Child 1 and create an isolated two-parent upstream merge commit.
3. Execute Children 2–5 in the fixed order and in reviewable commits; keep each child's lock/config changes scoped.
4. Execute Child 6 after task names, workflow structure, and checks are stable; archive only the five old tasks as `superseded` at this stage.
5. Run the full local gate, push the integration branch, open the implementation PR, require automatic CI, and merge with a GitHub merge commit.
6. Wait for the exact `main` SHA to pass `CI / Required`, run unsigned full-matrix preflight, create immutable `v0.3.0`, and verify the stable ten-asset Release, manifest, metadata, attestations, and unsigned status.
7. Create and merge the closeout PR containing real run/Release/digest/attestation evidence and refreshed design-package manifest.
8. Resolve every NO-GO condition, archive remaining children then parent, and record the Trellis journal after archive commits.

Rollback is by child/commit. Reverting the upstream merge is considered only before dependent commits or through an explicit coordinated revert series. After public `v0.3.0`, the tag and Release are immutable; defects move to a patch version.
