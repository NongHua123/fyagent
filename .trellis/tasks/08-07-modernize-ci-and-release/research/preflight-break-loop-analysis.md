# Break-loop analysis: v0.3.0 native preflight failures

- Date: 2026-08-08
- Scope: three failed exact-main unsigned preflights, the interrupted local candidate, and the engineering changes required before any further full preflight
- Status: research complete; implementation and native CI evidence pending

## Decision

Do not run another full five-target preflight as a diagnostic loop. The three
failed preflights show that the full release workflow has been acting as the
first integration test for lower-level runner, packaging, Windows Installer,
and metadata interfaces. That is too late and too expensive.

Before another preflight, implement two engineering boundaries and shift their
behavioral verification into normal Required CI:

1. Replace the Windows MSI verifier's ad hoc Automation reader with one
   schema-driven query module. It owns SQL construction, parameter binding,
   result-shape/type/null validation, resource limits, and deterministic COM
   lifetime. Exercise that production module against a temporary MSI generated
   from checked-in `.idt` source on native Windows x64 and ARM64 runners.
2. Replace undocumented hosted-image environment variables with a
   source-explicit metadata contract. Separate requested runner routing,
   documented runner-context observations, configured OCI image evidence, and
   container-observed OS/architecture facts. Validate exact nested key sets and
   test the writer as a process, including hostile ambient variables.

The interrupted candidate that merely removes `FieldCount` and
`ImageOS`/`ImageVersion` is useful diagnostic work but is not accepted as the
final implementation. It has no native Windows behavioral proof, retains raw
query ownership in the verifier, and does not make metadata provenance sources
unambiguous.

## Failure timeline

| Exact-main preflight | Failure boundary                                                                                         | What it proved                                                                                                                                                                 | What it did not prove                                                                          |
| -------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| `31238817378`        | Linux container bootstrap/tool ordering and Git safe-directory ownership; Windows SDK `mt.exe` discovery | Native runner routing worked far enough to expose workflow bootstrap assumptions.                                                                                              | Package generation, MSI verification, aggregation, attestation, or publication.                |
| `31241064177`        | Linux AppImage packaging/runtime extraction; Windows WiX Light diagnostics and MSI authoring             | The first remediation moved both platforms past bootstrap and exposed package-layer failures.                                                                                  | A complete five-target artifact set or release metadata.                                       |
| `31251654600`        | Windows MSI Automation query adapter; Linux job-container metadata writer                                | Windows x64/ARM64 both built MSIs before the same first query failed; Linux x64/ARM64 both built packages before the same undocumented metadata input failed. macOS completed. | The exact COM projection cause, aggregate metadata, attestation, publication, tag, or Release. |

All three runs failed closed. No formal tag or GitHub Release was created. The
formal release remains **NO-GO**.

## Root-cause classification

### B: cross-layer contract errors

- The release workflow treated runner-image implementation variables as a
  portable job-container API.
- The PowerShell verifier coupled package acceptance to an incidental COM
  result-shape member instead of an explicit query schema.
- Metadata production, TypeScript declarations, aggregation, workflow inputs,
  and documentation did not share one source-explicit shape.

### C: propagation failures

- Container facts verified before the build were not carried into the emitted
  metadata record.
- Handwritten aggregate fixtures bypassed the writer process and therefore did
  not prove environment ownership or serialized shape.
- Windows MSI acceptance assertions were duplicated across raw queries instead
  of sharing one query adapter.

### D: missing behavioral coverage

- Linux-host static string tests could prove that a PowerShell script contained
  expected text but could not load `WindowsInstaller.Installer` or exercise COM
  projection and cleanup.
- The regular Required CI Windows job compiled and tested Rust but did not
  execute the MSI query layer on either native architecture.
- The metadata writer lacked direct CLI tests with exact environment input,
  output-file semantics, and hostile ambient variables.

### E: implicit assumptions

- `ImageOS` and `ImageVersion` were assumed to cross the VM-to-job-container
  boundary and to describe the build user space. Neither assumption belongs to
  GitHub's documented workflow contract.
- `Record.FieldCount` was assumed to be a stable PowerShell/COM projection
  boundary on both runner architectures. The logs establish the observed zero,
  not a confirmed PowerShell, Windows Installer, or architecture-specific bug.
- A successful static gate was treated as sufficient evidence for a native
  interface that the gate never executed.

## Why the previous approach repeated the loop

1. Each remediation targeted the currently visible symptom and then used the
   expensive full matrix as the next discovery tool.
2. Static source assertions overfit implementation text and could pass without
   executing the native interface they claimed to protect.
3. Native diagnostics were colocated after full application/package builds,
   so cheap contract failures consumed the entire runner setup and build cost.
4. The metadata schema recorded values without preserving whether they were
   requested configuration, documented runtime context, or in-container
   observations.
5. Compatibility ideas such as invoking `FieldCount` differently, reflecting
   the same member, accepting zero with a fallback count, or retaining missing
   image values as null/sentinel would make the current error disappear without
   establishing a durable contract.

## Engineering option assessment

### Windows Installer query layer

| Option                                                     | Result                                                                                                                                                                   | Decision                                                                                                            |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Change `$record.FieldCount` syntax or use reflection       | Preserves dependence on the same unproven projection and has no authoritative evidence for the observed failure.                                                         | Reject except optional diagnostics.                                                                                 |
| Supply only an expected integer column count               | Avoids the immediate member but lets SQL, types, nullability, and reader shape drift independently.                                                                      | Reject as final design.                                                                                             |
| Add a zero-count compatibility fallback                    | Converts an unexplained native result into acceptance and can hide malformed queries.                                                                                    | Reject.                                                                                                             |
| Rewrite all access through MSI native P/Invoke immediately | Gives explicit native handles but substantially expands encoding, buffer, ownership, and error-surface obligations before broader Automation instability is established. | Reserve for a separate migration if the native fixture proves Automation is unstable beyond the removed projection. |
| Schema-driven Automation module plus native fixture        | Removes the unnecessary member, closes interpolation/type/null/resource/ownership gaps, and is testable on the authoritative native interfaces.                          | Adopt.                                                                                                              |

The adopted module must:

- accept only code-owned table/column identifiers and construct ordered
  projections from schema descriptors;
- bind all runtime values with `?` markers and `Installer.CreateRecord`;
- validate `View.ColumnInfo` names and MSI types by known ordinals without
  reading `FieldCount`;
- call `IsNull` and `DataSize` before typed `StringData`/`IntegerData` access,
  preserving null, empty string, and integer zero as distinct values;
- enforce per-field, row, aggregate-cell/unit, and stream-byte limits before
  materialization;
- expose copied primitive values only; no database/view/record/installer RCW
  may escape the module;
- close records, views, databases, and the installer in reverse ownership order,
  preserving the primary error while treating a cleanup-only failure as a
  verification failure;
- keep package-specific business assertions in the verifier rather than moving
  them into the generic query module.

### Runner/container metadata

| Option                                                     | Result                                                                                                            | Decision                   |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | -------------------------- |
| Keep `ImageOS`/`ImageVersion` required                     | Relies on undocumented hosted-image implementation details and describes the wrong OS layer for container builds. | Reject.                    |
| Keep them as null/sentinel or rename them                  | Conflates unavailable, not applicable, and failed collection while retaining a false provenance promise.          | Reject.                    |
| Remove them but retain ambiguous flat runner/digest fields | Fixes the current crash but leaves requested-versus-observed semantics unclear.                                   | Emergency-only, not final. |
| Structured source-explicit runner/container record         | Makes every value auditable and enables exact cross-validation without invented evidence.                         | Adopt.                     |

The adopted platform record must contain:

- `runner.requestedLabel`: exact workflow matrix routing request;
- `runner.context.os` and `runner.context.arch`: explicit workflow-owned inputs
  from documented `${{ runner.os }}` and `${{ runner.arch }}`;
- for Linux, `container.configuredImage.reference` and
  `.manifestDigest`: the exact digest-pinned OCI child requested by the job;
- for Linux, `container.observed.osRelease.id`, `.versionId`, and
  `.unameMachine`: facts measured inside the job container;
- for Windows/macOS, exactly `container: null`;
- exact allowlisted keys at every nested level and canonical reconstruction by
  the aggregate validator.

Do not emit `verified`, `actualImageDigest`, or a guessed hosted-image version.
Observed `/etc/os-release` and `uname -m` corroborate the configured native
boundary but do not independently prove an OCI digest. Attestation binds the
metadata bytes and workflow provenance; it does not certify that arbitrary
custom JSON claims are semantically true.

The failed preflight did not aggregate, attest, or publish the draft v1 shape.
A 2026-08-08 read-only inventory found no local `v0.3.0` tag and no GitHub
Release of any tag; repository search found the v1 identifiers only in the
current writer, validator, declarations, tests, and design contract. Therefore
finalize the unpublished per-platform and aggregate v1 schemas in place. If a
future public consumer is discovered before publication, bump both schemas
atomically to v2; never synthesize missing v1 fields.

## Shift-left prevention design

### Required PR/main CI

Add an unconditional `windows-msi-query` required job with a native matrix:

- `windows-2022` / x64;
- `windows-11-arm` / ARM64.

The job performs no frontend, Rust, application, MSI bundle, or signing build.
It checks out read-only, creates a temporary MSI database from reviewable `.idt`
fixture files, runs the production query module's success and failure cases,
and proves the temporary database can be renamed/deleted after both paths. It
records PowerShell version, Windows build/architecture, `msi.dll` version, and
runner diagnostic context. `windows-11-arm` is a public-preview runner; runner
unavailability fails closed and must not cause local cross-compilation or an
x64-only fallback.

Add this job ID to the exact Required dependency set. A missing, skipped,
cancelled, or failed matrix leg makes `CI / Required` fail.

### Metadata writer and aggregate tests

Add a direct CLI test suite that:

- invokes the real writer with one table-driven environment for each of the
  five target groups;
- validates exact serialized objects and write-once output behavior;
- covers missing, blank, partial, contradictory, malformed, unknown-key, and
  output-exists failures;
- poisons ambient `RUNNER_OS`, `RUNNER_ARCH`, `ImageOS`, and `ImageVersion` and
  proves that only workflow-owned inputs affect output;
- feeds writer output into the aggregate validator instead of relying only on
  handwritten fixtures;
- verifies that unknown keys are rejected and never spread into attested
  aggregate metadata.

### Full release preflight

The full preflight remains responsible for facts that the lightweight gates
cannot establish: real application builds, real packages on five native target
groups, exact 10 installer assets, exact 12 attestation subjects, the bundle as
the 13th attachment, and native unsigned/package-structure evidence. It must no
longer be the first execution of the MSI query adapter or metadata writer.

## Stop conditions before another preflight

No new `workflow_dispatch` release run is allowed until all conditions below
are true:

1. This analysis and both detailed research reports are reviewed with the
   implementation diff.
2. The interrupted compatibility candidate is replaced by the adopted designs;
   no `FieldCount` acceptance branch or `ImageOS`/`ImageVersion` compatibility
   field remains.
3. Linux x64 host-local formatting, type checking, contract/unit tests, task
   validation, and Trellis validation pass without foreign-target execution.
4. An independent Trellis check reviews the frozen diff and reports no
   unresolved high-confidence correctness or security finding.
5. The implementation PR's complete CI run is synchronously awaited until
   `completed` and read once; every required job, including both native Windows
   MSI fixture legs, succeeds.
6. After merge, the exact-main CI run is awaited and read under the same D117
   rule and succeeds.
7. Only then may the exact main/workflow SHA run the full unsigned preflight.

If that preflight fails, capture its final run/job table and failed logs once,
classify whether the failure is a new package/platform boundary or a regression
of a shifted-left contract, and return to the smallest authoritative test layer.
Do not repeatedly poll or use another full preflight to experiment.

## Durable knowledge capture

Implementation must update the active GitHub CI, GitHub Release, Windows
release, version/release, and task-runner boundaries plus their contract tests.
The v0.3.0 CI/Release design and exact asset contract must describe the same
source-explicit metadata and native query fixture. There is no generic reusable
repository template that owns these FyAgent-specific MSI/metadata contracts, so
no template payload is synchronized; the durable sources are the active specs,
workflow, production query/writer modules, and executable tests.

## Primary sources

- GitHub: [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- GitHub: [Variables reference](https://docs.github.com/en/actions/reference/workflows-and-actions/variables)
- GitHub: [Choosing the runner for a job](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job)
- GitHub: [Running jobs in a container](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container)
- Microsoft: [Record.FieldCount](https://learn.microsoft.com/en-us/windows/win32/msi/record-fieldcount)
- Microsoft: [View.ColumnInfo](https://learn.microsoft.com/en-us/windows/win32/msi/view-columninfo)
- Microsoft: [Record.IsNull](https://learn.microsoft.com/en-us/windows/win32/msi/record-isnull)
- Microsoft: [Record.DataSize](https://learn.microsoft.com/en-us/windows/win32/msi/record-datasize)
- Microsoft: [SQL syntax](https://learn.microsoft.com/en-us/windows/win32/msi/sql-syntax)
- Microsoft: [Database.Import](https://learn.microsoft.com/en-us/windows/win32/msi/database-import)
- Microsoft: [Windows Installer Automation interface](https://learn.microsoft.com/en-us/windows/win32/msi/automation-interface)

## Remaining evidence limits

- The exact cause of `FieldCount=0` is intentionally not claimed. The adopted
  design removes that unnecessary dependency and the native fixture proves the
  supported query behavior instead of guessing the binder bug.
- No local PowerShell, Windows Installer, WiX, macOS, ARM64, or foreign-target
  command is permitted. Native fixture and package evidence remain remote.
- The two retained Ubuntu 22.04 child digests' availability on the current
  hosted runner pools remains a full-preflight gate; changing them is a
  separate reviewed supply-chain update.
- Formal tag, Release, assets, attestation, independent downloads, closeout,
  task archival, and final branch cleanup remain pending and **NO-GO**.

## Outcome / Closure

The statements above preserve the evidence boundary at research time. The
recommended shift-left design was subsequently implemented and validated:

- corrected PR Required CI `31258884239` passed the native Windows x64/ARM64
  MSI query fixtures;
- exact-main Required CI `31259389682` passed;
- the next full five-target preflight `31259905022` completed successfully;
- formal run `31260931509` published stable Release ID `367220197` with exactly
  10 installers, 3 evidence attachments, and 12 attestation subjects;
- independent downloads, manifest/metadata comparison, and all 12 attestation
  verifications passed.

This closes the Release NO-GO without changing the original root-cause
discipline: the unproved `FieldCount=0` mechanism is still not asserted. The
remaining work is Trellis closeout, including the separate Windows ARM64
uv/Python/Trellis native smoke introduced after Release.
