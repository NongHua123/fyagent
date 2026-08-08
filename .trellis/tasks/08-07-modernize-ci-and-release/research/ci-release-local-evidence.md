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
  or Release permission separation. It did not substitute for the pending
  native Windows x64/ARM64 fixture evidence.
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

## Remote evidence so far

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
- All three preflights failed before asset aggregation, attestation, or
  publication; none created the immutable tag or public Release. D118 therefore
  blocks another full preflight until the query and metadata interfaces have
  engineering abstractions, behavior-level tests, and native Required evidence.
  D117 still requires the initiating flow to wait synchronously for each whole
  run to complete, then read the final state once; failed job logs may be
  fetched only after a final failure.

## Evidence still required before task completion

The child remains `in_progress`. The following are not satisfied by local/static evidence:

- a successful D118 engineering-remediation PR and exact-main `CI / Required`
  run, including the native Windows Installer query fixture on x64 and ARM64;
- a successful unsigned full-matrix preflight for the exact final main SHA;
- Windows x64/ARM64, Linux x64/ARM64, and macOS Universal native runner/package evidence;
- mandatory GitHub artifact attestations and Sigstore bundle verification;
- the exact stable `v0.3.0` GitHub Release and independent post-download validation;
- a successful manual preflight result required by the D114 substitute
  acceptance contract; PR/main and automatic Labeler evidence already exist.

The originally requested pre-merge same-SHA preflight is structurally incompatible with a GitHub merge commit plus truthful standard attestation provenance. The implemented safe order is merge -> successful main CI -> exact-main-SHA unsigned preflight -> tag -> formal Release.

The project owner accepted D113/D114 on 2026-08-08. D113 confirms that order.
D114 confirms a governance verification exception: the personal-account
repository and no-protection decision cannot enable GitHub Merge Queue, so a
real `merge_group` run remains impossible and is N/A, not successful. Its
accepted substitute is the YAML trigger, fail-closed contract/static tests,
and real PR/main/manual runs. This acceptance does not satisfy any pending
preflight, tag, Release, asset, attestation, or closeout evidence.

Formal Release remains **NO-GO** until the current local remediation is merged,
its exact main SHA passes Required CI and the complete five-target unsigned
preflight, and all downstream eligibility gates succeed.
