# CI and unsigned Release local evidence

Date: 2026-08-08
Branch: `codex/fyagent-v0.3.0-preflight-engineering`

## Implemented commits

- `038675b3` — restore automatic Required CI and safe automatic/manual labeling.
- `94ff9ee9` — implement the unsigned five-target Release, exact asset/evidence contracts, mandatory attestations, and fail-closed private-draft publication transaction.
- `6efdd6ad` — harden Linux packaging diagnostics, Windows MSI authoring,
  installer-action state classification, and rendered-MSI verification after the
  first two exact-main preflights failed closed.
- `a1c1238c` — enforce D116 host-native-only local execution and D117
  synchronous whole-run Actions waiting across the canonical task API and
  active contracts.
- `387f7fb8` — integrate the packaging and host-native remediation through PR
  #6 after its PR and exact-main Required CI passed. The third unsigned
  preflight then failed closed at the Windows Installer query adapter and
  Linux runner/container metadata boundary, before aggregation or publication.
- `d0af898a` — replace those low-level boundaries with the schema-owned Windows
  Installer query module, native x64/ARM64 fixture, and documented
  runner/container metadata contract submitted in PR #7.
- `d8c26b70` — permit the cleanup helpers' typed accumulator to begin empty
  while preserving deterministic COM release and aggregated cleanup failures;
  corrected PR #7 Required CI then passed.
- `bde1370bbaffd345c3d9875708615eaf96140591` — merge PR #7 to `main`; this is
  the exact source later used by main CI, preflight, annotated tag, and formal
  Release.

The public repository label prerequisite was also verified out of band: all nine labels referenced by `.github/labeler.yml` exist in `NongHua123/fyagent`; the previously missing `i18n` label was created with color `ededed` and description `Internationalization and localization`. The workflow retains only `pull-requests: write` and does not receive `issues: write`.

## Local verification

- CI/Labeler contract suite: 21 tests passed.
- CI-safe test selection with `mise` removed from `PATH`: 42 contract tests and 898 application tests passed; TypeScript passed.
- Full local application tests after the CI change: 137 files / 936 tests passed on the clean rerun.
- Release contract suite: 13 files / 100 tests passed.
- Additional focused Release suite: 4 files / 40 tests passed.
- `mise run typecheck`, `mise run format:check`, `mise run tasks:validate`, Trellis task validation, and `git diff --check` passed.
- `actionlint` 1.7.12 passed for both CI and Release workflows; its downloaded archive was checked against the official checksum before use.
- All three new Windows PowerShell verifiers passed Windows PowerShell AST parsing. Native Windows Installer COM/CAB execution remains a remote-runner gate.
- The final Release staged diff received an independent Trellis check and a cached-only security review. The reviewed diff SHA-256 was `69806311575310edc74bce7aa921a58a49bc572e5301c595e0801e2cb1fa8946`; the security review reported no remaining high-confidence Critical, High, Medium, or Low finding.
- The packaging/host-native remediation index received an independent frozen
  review at SHA-256
  `1c9529277366ab6e1869ca08c090bac31a353b6221e6705001f77ce6e13bdf32`.
  Its focused suites passed 57/57 TypeScript tests and 9/9 Rust tests, with no
  remaining high-confidence code, security, or supply-chain finding.
- The final Linux x64 host-native aggregate `mise run check` passed, including
  14 contract files / 144 tests and the Native Fetch 4/4 probe. Exact
  `v0.3.0` version checking and the active Child 4 Trellis validation also
  passed.
- The D118 engineering candidate passed `mise run release:check`: 16 contract
  files / 231 tests plus the Native Fetch 4/4 probe. This includes 49 direct
  writer CLI cases, 40 aggregate metadata cases, seven Windows Installer query
  static-contract cases, the Release workflow suite, the Required dependency
  gate, and the host-native boundary suite.
- The focused D118 matrix passed 132/132 tests. The final Linux x64
  `mise run check` completed with exit 0 after the metadata and sequence
  hardening, covering the full frontend, Rust, task, lock, hook, version, and
  Release aggregate without running a non-host target.
- Independent code-quality review found three concrete gaps: nullable MSI
  sequence rows could be coerced to zero, `macos-15` architecture was not exact,
  and the native COM fixture inherited GitHub's six-hour job timeout. The
  candidate now requires a positive integer sequence, freezes `macos-15` to
  `ARM64`, and limits each fixture job to 15 minutes; the focused suites and
  full local gate passed after those corrections.
- A separate read-only security review of the final D118 worktree found no new
  high-confidence finding in SQL parameterization, COM ownership, bounded
  stream handling, MSI business gates, Required topology, metadata allowlists,
  or Release permission separation. At review time it did not substitute for
  native Windows x64/ARM64 fixture evidence; those fixtures later passed in PR
  and exact-main Required CI as recorded below.
- `mise run typecheck`, targeted Prettier, task metadata/docs validation, and
  explicit parent/Child 4/Child 6 Trellis validation passed for the D118
  candidate. `actionlint` 1.7.12 also passed both changed workflows; the retained
  archive still matches the previously recorded official release checksum file.
- No local PowerShell, Windows Installer Automation, WiX, foreign target, or
  GitHub Actions command was used for D118. The x64/ARM64 temporary-MSI suite is
  intentionally a new Required CI gate, not a local or full-preflight claim.

## Security and provenance decisions realized in code

- Formal release eligibility binds the exact tag/main source to the expected CI workflow, latest successful main-push run/attempt, unique `CI / Required` job, and corresponding GitHub Actions check-run.
- Manual preflight is restricted to the exact trusted `main` workflow/source SHA. This is intentionally post-merge: standard GitHub artifact attestation provenance binds `GITHUB_SHA`, so an unmerged candidate built by a main workflow cannot truthfully be represented as standard provenance for that candidate SHA.
- Release workflows use no dependency cache, no signing/notarization secrets, no Release environment, no QEMU/cross-build fallback, and no persisted checkout credentials.
- Windows validation binds the built executable and the MSI-contained executable by size, SHA-256, PE Machine, and `NotSigned` state, in addition to the installer-actions and MSI structural contracts.
- Publication stages a private draft, verifies the exact 13 remotely downloaded attachments, and performs one final PATCH to stable/latest. Failure never deletes or retries the tag/Release; an ambiguous PATCH result is only read back and reported.

## Remote implementation and release evidence

- PR #4 head `4d3c32c1c60cb8be239aca7c3743abbe2aebacbf` passed CI run
  `31237734427`; merge commit
  `6301278b98470f6ade872b0e70e6967427df5a06` passed main CI run
  `31238303404`.
- PR #5 head `9b2e91983782577aafb2b18078f60e7bd405a95b` passed CI run
  `31240006190`; merge commit
  `6ed291b6c3e908d59a6b91cdd45714b5a34c7280` passed main CI run
  `31240470955`.
- PR #6 integrated the packaging and D116/D117 remediation as merge commit
  `387f7fb8a04b216b70590b37dfc8e0d034402588`; that exact main SHA passed CI
  run `31251170235`.
- Automatic Labeler run `31240006243` completed successfully with only
  `contents: read`, metadata read, and `pull-requests: write`; it applied the
  expected `frontend` and `actions` labels to PR #5.
- Exact-main unsigned preflight `31238817378` failed closed on Linux bootstrap
  and Windows SDK tool discovery; preflight `31241064177` failed closed on
  Linux AppImage packaging and Windows WiX Light.
- Exact-main unsigned preflight `31251654600` used
  `387f7fb8a04b216b70590b37dfc8e0d034402588`. macOS Universal succeeded.
  Windows x64 and ARM64 both completed application/helper/MSI builds and then
  failed at the first MSI structure query because the inline Automation reader
  reported `FieldCount=0`. Linux x64 and ARM64 both completed package builds
  and then failed because the metadata writer required undocumented `ImageOS`
  job-container state. Aggregate verification, attestation, and publication
  were skipped.
- D118 PR #7 first CI run `31258303784` waited synchronously to `completed`
  before one final result/log read. Repository contracts, frontend, all three
  backend jobs, and desktop acceptance succeeded. The new x64 and ARM64 MSI
  query fixture legs both failed immediately with the same PowerShell binder
  error: the typed cleanup-list parameter did not declare an empty collection
  as valid, so the first deterministic release call was skipped and the
  temporary MSI remained locked. `CI / Required` failed closed. The correction
  explicitly permits an empty cleanup accumulator on the three production
  cleanup helpers and the fixture owner; it does not ignore cleanup failures or
  add a compatibility fallback. No Release preflight was triggered.
- Corrected PR #7 head
  `d8c26b70c83fa6e4286d02549c0c383db4f5a318` passed run `31258884239`,
  including both native Windows Installer query fixtures and the fail-closed
  `CI / Required` aggregate. PR #7 merged as
  `bde1370bbaffd345c3d9875708615eaf96140591`.
- Exact-main run `31259389682` passed for that merge source, including native
  Windows x64/ARM64 query fixtures and `CI / Required`.
- Exact-main manual preflight `31259905022` passed eligibility, Windows x64,
  Windows ARM64, Linux x64, Linux ARM64, macOS Universal, exact installer and
  evidence aggregation, and attestation. Publish was correctly skipped in
  preflight mode.
- Annotated `v0.3.0` object
  `e6706d4bdc33a184cf641204574df1fc2962ca4c` peels to
  `bde1370bbaffd345c3d9875708615eaf96140591`.
- Formal tag run `31260931509` bound exact-main Required run `31259389682`,
  passed all five native build groups, verified the exact ten installers plus
  two generated evidence JSON files, attested all 12 subjects, and published
  the stable Release
  <https://github.com/NongHua123/fyagent/releases/tag/v0.3.0> (ID
  `367220197`). The Release is neither draft nor prerelease and is the latest
  Release.
- PR #8 <https://github.com/NongHua123/fyagent/pull/8> run `31264604075`, at
  head `623b6924e3b8682321b26aa69c15dc6f0b9f6f09`, failed closed after x64
  job `93120609402` passed. ARM64 job `93120609411` failed because setup-uv's
  version-only request selected `win-amd64`, so Required job `93121912798`
  failed. Commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` switched to a full
  uv request with managed Python. Run `31265504901` then passed x64 job
  `93122857985`, ARM64 job `93122858012`, and Required job `93123992476`.

The exact 13 published attachments are:

```text
FyAgent-0.3.0-macOS.dmg
FyAgent-0.3.0-macOS.zip
FyAgent-0.3.0-Windows.msi
FyAgent-0.3.0-Windows-arm64.msi
FyAgent-0.3.0-Linux-x86_64.AppImage
FyAgent-0.3.0-Linux-x86_64.deb
FyAgent-0.3.0-Linux-x86_64.rpm
FyAgent-0.3.0-Linux-arm64.AppImage
FyAgent-0.3.0-Linux-arm64.deb
FyAgent-0.3.0-Linux-arm64.rpm
download-manifest.json
build-metadata.json
artifact-attestation.sigstore.json
```

Independent verification downloaded all 13 attachments, enforced the exact
allowlist, and matched the ten installer names, byte sizes, SHA-256 values, and
URLs to `download-manifest.json`. Metadata matched the repository, workflow,
source/tag, formal run, and exact same-SHA Required run/job. Evidence SHA-256
values are:

```text
download-manifest.json                 d1d81b973aea506d369e21b385ee60b993b88121b946cf68dc254e825b4abea1
build-metadata.json                    7ae0631b77059d05a8866ec9602f8afc7f8493a092f6c32a3e4a161a3fc98079
artifact-attestation.sigstore.json     4802f1e9b5eca3eb0cc2a03530b86057d79e9d5828a97615d8ea5e5430ce0576
```

GitHub CLI `2.97.0` was independently downloaded and matched against its
official checksum. It verified all 12 attestation subjects with repository
`NongHua123/fyagent`, signer workflow
`NongHua123/fyagent/.github/workflows/release.yml`, source digest
`bde1370bbaffd345c3d9875708615eaf96140591`, source ref
`refs/tags/v0.3.0`, and the hosted-runner requirement bound. The temporary
installer and CLI downloads were removed after verification.

## D117 observation evidence

The initiating main flow synchronously waited for each whole run to reach
`completed` before one final result/job read:

- PR #7 first failure `31258303784`;
- corrected PR #7 success `31258884239`;
- exact-main Required success `31259389682`;
- exact-main preflight success `31259905022`;
- formal tag Release success `31260931509`.

Failed-job logs were retrieved only after `31258303784` had a final failure.
There was no background/asynchronous watcher, high-frequency status polling,
or progress-output loop.

## Closeout native evidence and remaining stages

The release transaction and all Release acceptance evidence are complete; the
release gate is **GO**. The PR #8 evidence above completes the native Windows
x64/ARM64 locked uv/Python and Trellis gate: corrected run `31265504901` passed
both native legs and aggregate Required. The same PR has rebuilt and verified
the final design-package manifest and must now archive Child 3, this Child 4,
Child 6, and parent in order, record the journal, and pass final PR CI before
merge.
Exact-main CI and final branch cleanup follow the merge and are also not claimed
by this record.

The originally requested pre-merge same-SHA preflight is structurally incompatible with a GitHub merge commit plus truthful standard attestation provenance. The implemented safe order is merge -> successful main CI -> exact-main-SHA unsigned preflight -> tag -> formal Release.

The project owner accepted D113/D114 on 2026-08-08. D113 confirms that order.
D114 confirms a governance verification exception: the personal-account
repository and no-protection decision cannot enable GitHub Merge Queue, so a
real `merge_group` run remains impossible and is N/A, not successful. Its
accepted substitute is the YAML trigger, fail-closed contract/static tests,
and real PR/main/manual runs. The substitute is now complete. The absence of
rulesets, branch protection, and a Release environment remains an explicitly
accepted governance residual and does not weaken the completed Required,
preflight, tag, asset, metadata, attestation, or publish evidence.
