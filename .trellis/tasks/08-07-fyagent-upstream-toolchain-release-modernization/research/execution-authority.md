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

## Executed implementation and release authority

- Implementation PR: <https://github.com/NongHua123/fyagent/pull/7>
- First PR Required CI: `31258303784` failed closed in both native MSI query
  fixtures because PowerShell rejected the empty typed cleanup accumulator.
- Corrected PR Required CI: `31258884239` passed, including native Windows x64
  and ARM64 fixtures. The correction explicitly permits an initially empty
  accumulator; it preserves deterministic COM release and aggregated cleanup
  failures rather than adding a compatibility fallback.
- Merged source: `bde1370bbaffd345c3d9875708615eaf96140591`.
- Exact-main Required CI: `31259389682` passed for that source.
- Exact-main unsigned preflight: `31259905022` passed eligibility, Windows x64,
  Windows ARM64, Linux x64, Linux ARM64, macOS Universal, exact attachment
  aggregation, and attestation without publishing.
- Annotated tag `v0.3.0`: object
  `e6706d4bdc33a184cf641204574df1fc2962ca4c`, peeled to the exact merged source.
- Formal tag run: `31260931509` passed every build, verification, attestation,
  and publish stage.
- Stable Release: <https://github.com/NongHua123/fyagent/releases/tag/v0.3.0>
  (database ID `367220197`), published with exactly ten unsigned installers and
  `download-manifest.json`, `build-metadata.json`, and
  `artifact-attestation.sigstore.json`.

Independent closeout verification downloaded all 13 attachments, rejected any
name outside the exact allowlist, compared every installer byte size and
SHA-256 to the manifest, and bound metadata to source
`bde1370bbaffd345c3d9875708615eaf96140591`, Required run `31259389682`, and
formal run `31260931509`. Evidence attachment SHA-256 values are:

```text
download-manifest.json                 d1d81b973aea506d369e21b385ee60b993b88121b946cf68dc254e825b4abea1
build-metadata.json                    7ae0631b77059d05a8866ec9602f8afc7f8493a092f6c32a3e4a161a3fc98079
artifact-attestation.sigstore.json     4802f1e9b5eca3eb0cc2a03530b86057d79e9d5828a97615d8ea5e5430ce0576
```

GitHub CLI `2.97.0`, itself checked against the official checksum, verified all
12 subjects in the published Sigstore bundle with the repository, signer
workflow, source digest, tag ref, and hosted-runner restrictions bound. The ten
installers plus manifest and metadata are the subjects; the bundle is not
self-attested.

D117 was exercised for the first failed PR run, corrected PR run, exact-main
run, preflight, and formal run. In each case the initiating flow synchronously
waited for the whole run to reach `completed` and then performed one final
run/job read. Failed-job logs were retrieved only after the first PR run had a
final failure. There was no background watcher or repeated status polling.

The only accepted governance residual remains D114: a live `merge_group` run is
N/A in this personal repository while rulesets, branch protection, and a
Release environment remain intentionally absent. This does not weaken any
Required, preflight, asset, digest, metadata, attestation, or publication gate.
Release evidence is complete and the release gate is **GO**. PR #8
<https://github.com/NongHua123/fyagent/pull/8> extends both Windows Required
legs with locked uv/Python setup and a Trellis task-list smoke before their MSI
fixture. Its first run `31264604075`, at head
`623b6924e3b8682321b26aa69c15dc6f0b9f6f09`, failed closed: x64 job
`93120609402` passed, ARM64 job `93120609411` failed because setup-uv's
version-only request selected `win-amd64`, and Required job `93121912798`
failed. Commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` switched the
workflow to a full uv request with managed Python. Run `31265504901` then passed
x64 job `93122857985`, ARM64 job `93122858012`, and Required job
`93123992476`. This completes the native closeout evidence and the Child 3
Windows ARM64 acceptance gate.

The same PR has rebuilt and verified the final design-package manifest and must
now archive Child 3, Child 4, Child 6, then parent, and record the journal. Only
after that series passes final PR CI may it merge; exact-main CI must then pass
before the explicitly authorized final branch cleanup. Those later actions are
not claimed here. D114 remains N/A, not successful.
