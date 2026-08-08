# CI and unsigned Release local evidence

Date: 2026-08-08
Branch: `codex/fyagent-v0.3.0`

## Implemented commits

- `038675b3` — restore automatic Required CI and safe automatic/manual labeling.
- `94ff9ee9` — implement the unsigned five-target Release, exact asset/evidence contracts, mandatory attestations, and fail-closed private-draft publication transaction.

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

## Security and provenance decisions realized in code

- Formal release eligibility binds the exact tag/main source to the expected CI workflow, latest successful main-push run/attempt, unique `CI / Required` job, and corresponding GitHub Actions check-run.
- Manual preflight is restricted to the exact trusted `main` workflow/source SHA. This is intentionally post-merge: standard GitHub artifact attestation provenance binds `GITHUB_SHA`, so an unmerged candidate built by a main workflow cannot truthfully be represented as standard provenance for that candidate SHA.
- Release workflows use no dependency cache, no signing/notarization secrets, no Release environment, no QEMU/cross-build fallback, and no persisted checkout credentials.
- Windows validation binds the built executable and the MSI-contained executable by size, SHA-256, PE Machine, and `NotSigned` state, in addition to the installer-actions and MSI structural contracts.
- Publication stages a private draft, verifies the exact 13 remotely downloaded attachments, and performs one final PATCH to stable/latest. Failure never deletes or retries the tag/Release; an ambiguous PATCH result is only read back and reported.

## Evidence still required before task completion

The child remains `in_progress`. The following are not satisfied by local/static evidence:

- real PR and main `CI / Required` runs;
- real automatic Labeler evidence after the new workflow exists on `main`;
- the unsigned full-matrix preflight for the exact final main SHA;
- Windows x64/ARM64, Linux x64/ARM64, and macOS Universal native runner/package evidence;
- mandatory GitHub artifact attestations and Sigstore bundle verification;
- the exact stable `v0.3.0` GitHub Release and independent post-download validation;
- real PR/main/manual evidence required by the D114 substitute acceptance contract.

The originally requested pre-merge same-SHA preflight is structurally incompatible with a GitHub merge commit plus truthful standard attestation provenance. The implemented safe order is merge -> successful main CI -> exact-main-SHA unsigned preflight -> tag -> formal Release.

The project owner accepted D113/D114 on 2026-08-08. D113 confirms that order.
D114 confirms a governance verification exception: the personal-account
repository and no-protection decision cannot enable GitHub Merge Queue, so a
real `merge_group` run remains impossible and is N/A, not successful. Its
accepted substitute is the YAML trigger, fail-closed contract/static tests,
and real PR/main/manual runs. This acceptance does not satisfy any pending
preflight, tag, Release, asset, attestation, or closeout evidence.
