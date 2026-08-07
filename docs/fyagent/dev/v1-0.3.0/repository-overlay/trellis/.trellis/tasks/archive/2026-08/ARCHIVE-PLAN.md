
# Proposed Superseded-Task Archive

> **Status**: Proposed / 拟实施. This is a document-level result, not proof that the repository task command has run.

The five copied task directories show the proposed archive result for every task directory that actually exists in the uploaded baseline. In the real repository:

1. add `archiveDisposition = superseded`, `supersededBy`, `archiveReason`, and `historicalStatusBeforeArchive` before archival;
2. archive the four existing child directories first and the parent last;
3. invoke the existing task tool with `--no-commit`;
4. inspect the complete diff and create one explicit reviewable archive commit.

The Trellis tool's `status = completed` is only its technical closed state. The `meta` fields and each `ARCHIVE-NOTE.md` preserve the truthful project meaning: the work was **superseded**, not implemented to completion.

## Parent references that require verification

The parent task lists two child IDs that were not present as active task directories in the uploaded archive:

```text
08-06-fyagent-v1-0-2-shell-window
08-06-fyagent-v1-0-2-desktop-acceptance
```

Do not invent or delete those records during archival. In the real Git checkout, first determine whether they exist in Git history, a different worktree, or were removed before the supplied snapshot. Record the finding in the archive commit. This overlay intentionally archives only the five directories observed in the supplied baseline.
