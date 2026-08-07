# FyAgent v0.3.0 execution authority

## Verified baseline

- implementation baseline: `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`
- integration branch: `codex/fyagent-v0.3.0`
- PR base: `main`
- recovery ref: `refs/backup/fyagent-v0.3.0-baseline`
- upstream tag object: `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`
- upstream peeled commit: `43eaf07355af145aebfee301801779e824d4c221`
- public repository: `NongHua123/fyagent`

## Approved overrides to the original design package

The project owner approved real implementation and release on 2026-08-08. Product version is `0.3.0`; `v0.3.0` is an unsigned stable public Release with exactly ten installers, a SHA-256 manifest, build metadata, and GitHub artifact attestations. Windows/macOS signing and notarization are out of scope.

Automatic CI and a safe automatic Labeler are restored. No branch/tag rulesets, branch protection, or Release environment approval are configured; workflow-only source eligibility is an explicitly accepted residual risk. The implementation reaches `main` through a GitHub merge commit, preserving the internal upstream two-parent merge ancestry. A post-release closeout PR records real evidence before the new task tree is archived.

All product identity, mixed-license boundaries, `~/.fyagent`, `fyagent.db`, schema 16, backup behavior, deep links, bundle identity, and `FYAGENT_*` contracts remain unchanged.
