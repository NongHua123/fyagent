# Child 6 local documentation and Trellis evidence

Date: `2026-08-08`

## Evidence boundary

This record covers local documentation, long-lived specs, Trellis workflow and
skills, the frozen overlay boundary, public release documentation, the five old
superseded task archives, and the observed PR/main/Labeler/preflight/formal
Release outcomes listed below. It now records the published digests,
attestations, independent download verification, PR #8's expanded native
Windows uv/Python/Trellis result, and the verified final design-package
`MANIFEST.sha256`. It does not claim that the closeout archive/journal, final PR
CI/merge, exact-main CI, or branch cleanup has completed.

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
- `a1c1238c4f7ec8f80238edfb2618823bcedf49f5` — document and enforce D116
  host-native-only local execution plus D117 synchronous whole-run Actions
  waiting across the active task API, specs, and v0.3.0 design package.

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
- decision IDs D1–D118 are continuous and unique, with historical D1–D104
  semantics preserved;
- repository overlay accounting is 111 total files: 108 frozen payload files
  and three central freeze declarations; only those declarations changed;
- cached and worktree whitespace checks.

The active task remained
`.trellis/tasks/08-07-migrate-docs-and-trellis-specs` throughout the old-task
archive.

## Accepted decisions and completed remote evidence

- The project owner accepted D113/D114 on 2026-08-08. D113's post-merge
  exact-main/workflow-SHA order was executed. D114 remains a governance
  exception: a live `merge_group` event is impossible for the personal-account
  repository while protection/rulesets remain intentionally absent, so the
  live run is N/A rather than successful. Its YAML/static plus real
  PR/main/manual substitute is complete.
- D116 requires canonical local development, build, test, package, and
  verification entrypoints to execute only the current host OS/architecture;
  non-host evidence comes from matching native Actions runners. No local
  Windows, macOS, ARM64, foreign-target, or cross-OS acceptance command was
  used.
- D118 moved the low-level Windows Installer and runner/container metadata
  contracts behind engineering abstractions, behavior tests, and native
  x64/ARM64 Required CI fixtures. PR #7 run `31258303784` failed closed on an
  empty typed cleanup accumulator. The corrected run `31258884239` passed both
  fixtures and Required without ignoring cleanup failures or adding a fallback.
- PR #7 merged as `bde1370bbaffd345c3d9875708615eaf96140591`.
  Exact-main Required run `31259389682` passed. Exact-main preflight
  `31259905022` passed all five native release groups, aggregation, and
  attestation without publishing.
- Annotated tag object `e6706d4bdc33a184cf641204574df1fc2962ca4c`
  peels to that exact main source. Formal run `31260931509` passed eligibility,
  all builds, exact verification, attestation, and final publish.
- Stable Release
  <https://github.com/NongHua123/fyagent/releases/tag/v0.3.0> (ID
  `367220197`) is non-draft, non-prerelease, latest, and contains exactly ten
  unsigned installers plus `download-manifest.json`, `build-metadata.json`, and
  `artifact-attestation.sigstore.json`.
- PR #8 <https://github.com/NongHua123/fyagent/pull/8> run `31264604075`, at
  head `623b6924e3b8682321b26aa69c15dc6f0b9f6f09`, failed closed after x64
  job `93120609402` passed. ARM64 job `93120609411` failed because setup-uv's
  version-only request selected `win-amd64`, so Required job `93121912798`
  failed. Commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` switched to a full
  uv request with managed Python. Run `31265504901` then passed x64 job
  `93122857985`, ARM64 job `93122858012`, and Required job `93123992476`.

Independent verification downloaded all 13 attachments and matched the exact
allowlist, sizes, hashes, and URLs. It bound build metadata to source
`bde1370bbaffd345c3d9875708615eaf96140591`, main Required run
`31259389682`, and formal run `31260931509`. GitHub CLI `2.97.0`, checked
against its official checksum, verified all 12 bundle subjects with exact
repository, signer workflow, source digest, tag ref, and hosted-runner
restrictions. Evidence attachment SHA-256 values are:

```text
download-manifest.json                 d1d81b973aea506d369e21b385ee60b993b88121b946cf68dc254e825b4abea1
build-metadata.json                    7ae0631b77059d05a8866ec9602f8afc7f8493a092f6c32a3e4a161a3fc98079
artifact-attestation.sigstore.json     4802f1e9b5eca3eb0cc2a03530b86057d79e9d5828a97615d8ea5e5430ce0576
```

## D117 observation evidence

Runs `31258303784`, `31258884239`, `31259389682`, `31259905022`, and
`31260931509` were each handled by the initiating main flow with one
synchronous whole-run wait through `completed`, followed by one final run/job
result read. Failed-job logs were retrieved only after the first run's final
failure. No background/asynchronous monitor, high-frequency polling, or
progress-output loop was used.

## Closeout native evidence and remaining boundary

Formal Release evidence is complete and the release gate is **GO**. PR #8's
corrected run `31265504901` passed the locked uv/Python plus Trellis task-list
smoke on both native Windows Required legs and passed aggregate Required. The
original Windows ARM64 native-smoke criterion is complete. The final
design-package `MANIFEST.sha256` is rebuilt and verified. Ordered new-task
archives, journal, final PR CI/merge, exact-main CI, and final branch cleanup
remain pending and are not claimed here. The accepted absence of rulesets, branch protection, and
a Release environment remains the only governance residual; D114 is N/A, not
successful, and does not waive any closeout gate.
