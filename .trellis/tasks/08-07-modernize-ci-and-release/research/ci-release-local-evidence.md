# CI and unsigned Release local evidence

Date: 2026-08-08
Branch: `codex/fyagent-v0.3.0`

## Implemented commits

- `038675b3` — restore automatic Required CI and safe automatic/manual labeling.
- `94ff9ee9` — implement the unsigned five-target Release, exact asset/evidence contracts, mandatory attestations, and fail-closed private-draft publication transaction.
- `6efdd6ad` — harden Linux packaging diagnostics, Windows MSI authoring,
  installer-action state classification, and rendered-MSI verification after the
  first two exact-main preflights failed closed.
- `a1c1238c` — enforce D116 host-native-only local execution and D117
  synchronous whole-run Actions waiting across the canonical task API and
  active contracts.

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
- `actionlint` was not installed for the latest remediation revision, so its
  GitHub Actions semantic result remains a remote CI gate. Earlier workflow
  revisions passed actionlint 1.7.12, but that is not reported as evidence for
  the current revision.

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
- Automatic Labeler run `31240006243` completed successfully with only
  `contents: read`, metadata read, and `pull-requests: write`; it applied the
  expected `frontend` and `actions` labels to PR #5.
- Exact-main unsigned preflight `31238817378` failed closed on Linux bootstrap
  and Windows SDK tool discovery; preflight `31241064177` failed closed on
  Linux AppImage packaging and Windows WiX Light. Neither run reached asset
  aggregation, attestation, or publication, and neither created a tag or
  Release.
- The next remediation PR/main CI and preflight have not run. D117 requires the
  initiating flow to wait synchronously for each whole run to complete, then
  read the final state once; failed job logs may be fetched only after a final
  failure.

## Evidence still required before task completion

The child remains `in_progress`. The following are not satisfied by local/static evidence:

- a successful remediation PR and main `CI / Required` run for `6efdd6ad` and
  `a1c1238c`;
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
