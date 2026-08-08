# Parent Implementation Plan

1. Verify the exact baseline, remotes, upstream tag object/peeled SHA, recovery ref, integration branch, and public-repository state.
2. Complete Child 1 and create an isolated two-parent upstream merge commit.
3. Execute Children 2–5 in the fixed order and in reviewable commits; keep each child's lock/config changes scoped.
4. Execute Child 6 after task names, workflow structure, and checks are stable; archive only the five old tasks as `superseded` at this stage.
5. Run the full local gate, push the integration branch, open the implementation PR, require automatic CI, and merge with a GitHub merge commit.
6. [done] Wait for the exact `main` SHA to pass `CI / Required`, run unsigned full-matrix preflight, create immutable `v0.3.0`, and verify the stable ten-asset Release, manifest, metadata, attestations, and unsigned status.
7. [native evidence and final manifest complete; archive pending] Open the closeout PR
   with real run/Release/digest/attestation evidence and the final native Windows
   x64/ARM64 locked uv/Python/Trellis Required smoke. PR #8 first failed closed
   in run `31264604075`, was corrected by
   `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd`, and passed both native legs plus
   Required in run `31265504901`. The final design-package manifest is rebuilt
   and verified; archive the remaining children then parent in the same PR.
8. [pending] Record the Trellis journal after the archive commit, push the
   completed closeout series, require the final PR CI to pass, merge it, require
   exact-main CI to pass, and only then remove all writable local/origin branches
   except `main`.

Steps 1–5 completed before PR #7. Step 6 completed from exact source
`bde1370bbaffd345c3d9875708615eaf96140591`: main Required CI
`31259389682`, preflight `31259905022`, and formal Release run `31260931509`
all succeeded. The stable Release is
<https://github.com/NongHua123/fyagent/releases/tag/v0.3.0>. Step 7's native
gate is complete: run `31265504901` passed x64 job `93122857985`, ARM64 job
`93122858012`, and Required job `93123992476`; the final design-package
manifest is also rebuilt and verified. Ordered archives, journal, final PR
CI/merge, exact-main CI, and cleanup portions of Steps 7–8 remain pending and
are not claimed complete by this active record.

Rollback is by child/commit. Reverting the upstream merge is considered only before dependent commits or through an explicit coordinated revert series. After public `v0.3.0`, the tag and Release are immutable; defects move to a patch version.
