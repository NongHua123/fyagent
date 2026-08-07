# Child 6 local documentation and Trellis evidence

Date: `2026-08-08`

## Evidence boundary

This record covers local documentation, long-lived specs, Trellis workflow and
skills, the frozen overlay boundary, public release documentation, and the five
old superseded task archives. It does not claim any PR/main Actions run, native
release build, tag, GitHub Release, published digest, attestation, independent
download verification, final `MANIFEST.sha256`, remaining new-task archive, or
journal result.

## Work commits

- `eb748f9c86baca88bd7cea9b15957f9be5095790` — make canonical Trellis
  validation and CLI help honor their documented contracts.
- `58101230e634e2280ef24937fad1942ed3d3d75f` — migrate public documentation,
  long-lived specs, Trellis workflow/skills, decision tracking, and the v0.3.0
  design package.
- `1d3849e66b642c2282d388fec0467fc68af68244` — archive the five old tasks with
  `superseded` disposition while preserving their original status and history.
- `580c5efaacc0276ee0650e5778e9b778f506abaa` — prove the no-argument
  `trellis:validate` path against the post-archive active task set.

## Independent review

- Public documentation review initially found Release-body relative links,
  publication-state wording, historical CHANGELOG formatting damage, and one
  missing Chinese Windows download link. All four findings were fixed and the
  final public diff was accepted.
- Internal documentation review found canonical Python/bootstrap drift,
  `finish-work` naming, overlay count terminology, D113 visibility, and the
  Trellis help implementation boundary. All findings were fixed; the final
  39-file internal set had no remaining finding.
- The five-task superseded archive received an independent staged-diff review
  with no findings. The four child tasks were archived before the old parent;
  the already archived shell-window and desktop-acceptance tasks were not
  modified or duplicated.

## Local verification

The following completed successfully on Linux x64:

- generated task documentation is byte-for-byte current;
- all 80 mise task metadata and contract checks;
- explicit Child 6 Trellis context validation;
- no-argument `mise run trellis:validate` over all four active tasks;
- `tests/miseTaskContract.test.ts`: 12/12 tests;
- `mise run release:check`: 14 files / 108 tests;
- Native Fetch focused probe: 4/4 tests under pending+throw deprecation flags;
- `mise run check:contracts`;
- 39 internal files / 49 relative links and 56 combined live Markdown files /
  178 relative links, with no missing local target;
- decision IDs D1–D115 are continuous and unique, with historical D1–D104
  semantics preserved;
- repository overlay accounting is 111 total files: 108 frozen payload files
  and three central freeze declarations; only those declarations changed;
- cached and worktree whitespace checks.

The active task remained
`.trellis/tasks/08-07-migrate-docs-and-trellis-specs` throughout the old-task
archive.

## Remaining NO-GO and remote evidence

- D113: post-merge exact-main-SHA preflight is implemented but still requires
  project acceptance.
- D114: a real `merge_group` event is unavailable for the personal-account
  repository while branch protection/rulesets remain forbidden.
- PR/main CI, automatic Labeler, the five native release groups, exact-main
  unsigned preflight, immutable `v0.3.0`, the stable 13-attachment Release,
  digest/attestation verification, Windows `NotSigned`, macOS Developer ID and
  notarization absence, independent re-download, final manifest, closeout task
  archives, and journal remain pending.

Child 6 and the parent must remain active until these gates are resolved and the
real closeout evidence is committed.
