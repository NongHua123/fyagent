# Parent Implementation Plan

1. Verify real Git baseline, remotes, full upstream tag SHA, and create integration branch.
2. Complete Child 1 and create an isolated merge commit.
3. Rebase the implementation plan against the merged tree without changing confirmed product decisions.
4. Execute Children 2–5 in reviewable commits; keep each child's lockfile/config changes scoped.
5. Execute Child 6 after task names, workflow structure, and checks are stable.
6. Run Required CI and formal Release preflight according to the final contracts.
7. Resolve the risk register to GO or GO WITH CONDITIONS, record residual conditions, and close the parent.

Rollback is by child/commit. Reverting the upstream merge is considered only before dependent commits or through an explicit coordinated revert series.
