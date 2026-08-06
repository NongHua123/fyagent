# Desktop visual-baseline policy

This directory intentionally contains no accepted PNG baseline while local execution is
restricted to mock-only checks. A candidate runner must capture each region twice with
the fixed fixture, use fake IPC with external network blocked, and compare only against
the same platform, scale, and locale directory.

PNG files below this directory are tracked with Git LFS. Ordinary test commands may
compare an existing baseline but must never create, overwrite, or accept one. A reviewer
must run `mise exec -- pnpm test:desktop:visual:update` with explicit candidate evidence
before adding or replacing a baseline in a separate reviewed change.
