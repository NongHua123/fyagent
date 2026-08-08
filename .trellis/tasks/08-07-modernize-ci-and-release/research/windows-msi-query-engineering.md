# Research: Windows MSI query engineering

- Query: Investigate the native Windows x64/ARM64 `verify-windows-msi-structure.ps1:73 FieldCount=0` failure and select a robust long-term Windows Installer query design, native test strategy, and PR-CI prevention gate.
- Scope: mixed
- Date: 2026-08-08

## Findings

### Executive conclusion

The evidence proves a shared Windows Installer Automation query-adapter failure,
not its exact interop cause. Both native Windows jobs built a real MSI and then
failed at the same first one-column query because the verifier observed
`FieldCount=0`. Microsoft documents `Record.FieldCount` as a read-only Automation
**property**, while PowerShell documents parentheses as method invocation. No
authoritative Microsoft or PowerShell source found in this research reproduces
the exact zero result, and the failed run did not record its exact PowerShell,
runner-image, OS-build, or `msi.dll` version. Consequently, changing property
syntax, invoking it as a method, using reflection, or substituting an expected
count when zero would be an unproven compatibility branch rather than a root
design.

The robust long-term design is to make result shape an explicit, code-owned
query contract:

1. Construct every `SELECT` from an ordered schema of allowed MSI identifiers;
   do not accept `SELECT *` or caller-composed result lists.
2. Bind every runtime value with Windows Installer `?` parameters and a
   `Record` created by `Installer.CreateRecord`; runtime values never enter SQL
   text. Dynamic identifiers are not parameterizable and therefore must come
   from a closed code allowlist.
3. Validate the returned column names and MSI types by their known ordinals
   through `View.ColumnInfo`, without consulting `FieldCount`.
4. Read each known ordinal according to its declared type. Call `IsNull` first,
   call `DataSize` before materializing strings, use `StringData` only for
   strings and `IntegerData` only for integers, and return only copied
   `string`/`Int32`/`null`/`bool` values. No `Record`, `View`, database, installer,
   or other RCW may leave the query module.
5. Put this behavior and COM ownership in one script module, with separate
   scalar/unique-row, multi-row, and bounded-stream operations. A module alone
   is not the fix; it makes the contract testable and prevents a second raw COM
   reader from drifting.
6. Generate a tiny temporary MSI database from reviewable `.idt` fixture source
   on native Windows runners and exercise the production module on both
   `windows-2022` x64 and `windows-11-arm` ARM64 in regular required PR CI. This
   is the missing runtime evidence. Static TypeScript string assertions remain
   useful but cannot establish Automation behavior.

This is not merely a `FieldCount` avoidance workaround. It removes several
independent false-acceptance and resource risks already present in a raw string
reader: SQL interpolation, query/schema drift, null-versus-empty collapse,
integer-to-string coercion, post-allocation length checks, unbounded aggregate
materialization, live-COM escape, and cleanup failures reduced to warnings.

### Evidence boundary and failure classification

The observed baseline for this research is the recorded run at source
`387f7fb8a04b216b70590b37dfc8e0d034402588`, plus the two frozen `/tmp` reports.
The working tree contains a later interrupted candidate patch and is evaluated
only as a candidate, never as evidence that the failure is fixed.

| Observation                                                                                                                                 | Evidence                                                                               | Meaning                                                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| The recorded workflow was `workflow_dispatch`, ended `failure`, and used the exact baseline SHA.                                            | `/tmp/fyagent-preflight-final.json:1`                                                  | The reports belong to a concrete full-preflight candidate, not a local reconstruction.                                         |
| ARM64 completed executable build, manifest validation, installer-actions DLL validation, and MSI bundling before the structure step failed. | `/tmp/fyagent-preflight-final.json:1`; `/tmp/fyagent-preflight-failed.log:42-54`       | A real ARM64 MSI existed. The failure occurred in the post-bundle verifier before artifact normalization/upload.               |
| x64 completed the same preceding stages and failed in the same step.                                                                        | `/tmp/fyagent-preflight-final.json:1`; `/tmp/fyagent-preflight-failed.log:55-106`      | The failure is not isolated to the ARM64 runner.                                                                               |
| Both logs report line 73, `invalid field count 0`, for exactly `SELECT \`Name\` FROM \`Binary\` WHERE \`Name\`='FyAgentInstallerActions'`.  | `/tmp/fyagent-preflight-failed.log:49-54`, `/tmp/fyagent-preflight-failed.log:100-106` | The shared reader rejected the first one-column result shape before any architecture-specific table assertion could succeed.   |
| ARM64 resolved Windows SDK Manifest Tool `10.0.26100.0\arm64`; x64 resolved the matching x64 tool.                                          | `/tmp/fyagent-preflight-failed.log:48`, `/tmp/fyagent-preflight-failed.log:99`         | The preceding manifest check used the intended architecture. It does not identify the Windows Installer or PowerShell version. |
| The release workflow uses native `windows-2022` x64 and native `windows-11-arm` ARM64 matrix entries.                                       | `.github/workflows/release.yml:340-357`                                                | The two failures came from the same two runner families used for release.                                                      |

The strongest supported classification is **shared query-adapter contract
mismatch**. Because both architectures reached the same query and same result,
package corruption or an architecture-specific Binary-table absence is less
likely; that is an inference, not a proven exclusion. The logs do not show the
raw result of `Fetch`, a `ColumnInfo` record, the COM type library projection,
PowerShell version, `msi.dll` version, or an independent native MSI API query.
It would therefore be incorrect to label this a confirmed PowerShell 7 bug, a
Windows Installer bug, an ARM emulation problem, or a malformed MSI.

### Files found

| Path                                                   | Relevant role                                                                                                                                                                                     |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/tmp/fyagent-preflight-final.json`                    | Frozen full-preflight run/job/step outcome for baseline SHA `387f7fb8…`; both native Windows jobs fail only after MSI bundling.                                                                   |
| `/tmp/fyagent-preflight-failed.log`                    | Exact x64 and ARM64 error text, query, baseline script line, architecture-matched SDK tool, and shell path.                                                                                       |
| `scripts/release/verify-windows-msi-structure.ps1`     | Current interrupted candidate. It now supplies explicit `Columns`, but still owns raw SQL construction, string-only materialization, stream reading, and COM cleanup in the verifier.             |
| `.github/workflows/release.yml`                        | Builds real x64/ARM64 MSIs on native runners and invokes the structure verifier only after the expensive app/DLL/MSI build (`:340-357`, `:492-538`).                                              |
| `.github/workflows/ci.yml`                             | Regular PR/main CI has only the x64 `backend-windows` Rust job; it does not create a test MSI or exercise Windows Installer Automation (`:230-271`).                                              |
| `scripts/ci/required-gate.mjs`                         | Encodes the exact six required dependency IDs and fails on missing/extra/non-success results (`:6-69`). Any new MSI runtime job must be added here.                                               |
| `tests/requiredCiGate.test.ts`                         | Verifies exact required-job aggregation and fail-closed outcomes (`:22-92`).                                                                                                                      |
| `tests/ciWorkflow.test.ts`                             | Statically requires exactly six dependencies and fixed runner labels (`:10-17`, `:67-105`).                                                                                                       |
| `tests/releaseWorkflow.test.ts`                        | The interrupted candidate adds static checks for explicit `Columns` and absence of `.FieldCount`, but performs no COM call (`:945-996`).                                                          |
| `.trellis/spec/backend/github-ci-workflow.md`          | Current exact-six-job Required topology, unconditional execution, allowed runners, and test/evidence contract (`:45-72`, `:97-145`, `:268-295`).                                                  |
| `.trellis/spec/backend/github-release-workflow.md`     | Native release runner matrix and Windows MSI gates (`:133-152`, `:182-230`, `:353-369`).                                                                                                          |
| `.trellis/spec/backend/windows-release-boundary.md`    | Both-architecture and no-local-cross-OS boundary (`:12-14`, `:90-100`); interrupted candidate text currently describes explicit columns without native fixture evidence (`:211-218`, `:313-349`). |
| `.trellis/tasks/08-07-modernize-ci-and-release/prd.md` | Prohibits local cross-OS/cross-architecture verification and requires a real unsigned full matrix (`:26-45`).                                                                                     |

### Current candidate pattern: useful direction, incomplete engineering boundary

The interrupted working-tree candidate is directionally better than the failed
baseline but should not be accepted as the final abstraction without native
tests and additional hardening:

- `Assert-MsiQueryShape` takes `Columns`, caps the list at 16, checks unique
  identifier spelling, and compares a generated `SELECT` prefix
  (`verify-windows-msi-structure.ps1:38-64`). This removes the observed
  `FieldCount` read, but a string-prefix assertion keeps SQL and expected shape
  as two separately authored facts. It does not validate MSI column type or
  nullability.
- `Get-MsiRecords` reads exactly the expected ordinals and returns a copied
  `PSCustomObject` (`:85-137`). However, every value is first obtained through
  `StringData` into `string[]` (`:111-119`). This lets Windows Installer coerce
  integers to strings, collapses null and empty, and leaves callers to cast
  strings after the fact. A missing/nonexistent field can become an empty string
  under the documented Automation contract.
- The 1,024-UTF-16-unit check occurs **after** `StringData` has materialized the
  value (`:113-116`). `DataSize` exists specifically to obtain a size before
  reading. The candidate also has row caps (`:22-24`, `:93-109`) but no explicit
  aggregate cell/character budget.
- Runtime values remain interpolated into SQL, including action/property values
  and values read from the candidate MSI (`:244-245`, `:271-284`, `:293-313`).
  Some call sites apply useful identifier validation, but parameter binding is
  the Windows Installer-supported mechanism and removes the need to reason
  independently about every value's quoting.
- The stream reader validates a conservative name alphabet and reads in bounded
  chunks (`:175-224`), but accepts any positive declared stream size
  (`:192-205`). It needs an explicit total stream-byte ceiling before creating
  or filling the output file.
- `View.Close()` exceptions are warnings in both the tabular and stream paths
  (`:125-135`, `:226-240`). A verifier that otherwise reports success after a
  resource-close failure is not fail closed. Cleanup must preserve a primary
  query failure while still making a cleanup-only failure reject the candidate.
- Top-level code final-releases database and installer RCWs and then forces a
  global collection/finalizer cycle (`:708-719`). Deterministic ownership should
  make the global GC workaround unnecessary; a native fixture should prove the
  MSI can be immediately renamed/deleted after both success and failure.
- The current TypeScript test proves source-text patterns only
  (`tests/releaseWorkflow.test.ts:945-996`). It could remain a useful contract
  guard, but it could have passed on Linux without ever loading
  `WindowsInstaller.Installer`.

### Official contracts that govern the design

#### `FieldCount` syntax and why an invocation change is not a design

Microsoft's Windows Installer Automation reference defines
[`Record.FieldCount`](https://learn.microsoft.com/en-us/windows/win32/msi/record-fieldcount)
as a read-only property with syntax `propVal = Record.FieldCount`; it does not
document a zero-argument method. PowerShell's
[`about_Methods`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_methods)
states that method invocation uses parentheses. Therefore:

- `$record.FieldCount` matches the published Automation contract.
- `$record.FieldCount()` asks PowerShell to invoke a method and has no published
  Windows Installer basis.
- `InvokeMember(..., GetProperty, ...)` might be useful in a one-off native
  diagnostic to compare COM binder projections, but it retains the same
  metadata dependency and is not a production architecture.
- “If zero, use the expected number” is unsafe. The documented
  [`Record.StringData`](https://learn.microsoft.com/en-us/windows/win32/msi/record-stringdata)
  behavior returns an empty string for a nonexistent field, so fallback reads
  can turn a missing result field into plausible empty data instead of rejecting
  the package.

An optional native integration diagnostic may record direct property access and
reflection access so a future runner-image regression has evidence. Production
logic must neither branch on those observations nor require `FieldCount` to be
nonzero.

#### Shape, type, nullability, and size

- [`View.ColumnInfo`](https://learn.microsoft.com/en-us/windows/win32/msi/view-columninfo)
  returns a Record containing result-column names (`msiColumnInfoNames = 0`) or
  types (`msiColumnInfoTypes = 1`). A query layer that already knows `N` can
  compare ordinals `1..N` to its schema without reading `FieldCount`.
- The [Column Definition Format](https://learn.microsoft.com/en-us/windows/win32/msi/column-definition-format)
  encodes string versus 2-/4-byte integer, nullability, and string width. It
  explicitly notes that Windows Installer does not internally limit a string to
  its declared column width. The verifier must retain its own field limits.
- [`Record.IsNull`](https://learn.microsoft.com/en-us/windows/win32/msi/record-isnull)
  must be checked before selecting a value accessor. Null is not equivalent to
  an empty string or numeric zero.
- [`Record.DataSize`](https://learn.microsoft.com/en-us/windows/win32/msi/record-datasize)
  reports string length, stream byte count, four bytes for an integer, and zero
  for null. It supports a pre-materialization cap.
- [`Record.StringData`](https://learn.microsoft.com/en-us/windows/win32/msi/record-stringdata)
  converts integer/object fields to strings and returns empty for a nonexistent
  field. It must not be the universal accessor.
- [`Record.IntegerData`](https://learn.microsoft.com/en-us/windows/win32/msi/record-integerdata)
  returns a 32-bit integer and uses the MSI database-null integer for null or a
  value that cannot be converted. `IsNull` plus exact schema validation is
  required before accepting it.

Known-ordinal access is safe only when the code itself constructs the exact
`SELECT` list. If arbitrary raw SQL remains public, an expected column array is
still a parallel assertion and cannot prove that the query has no extra result
columns. The recommended query abstraction therefore generates the result list
from the descriptors and keeps the raw `OpenView` entry point module-private.

#### Parameters and restricted SQL

[`View.Execute`](https://learn.microsoft.com/en-us/windows/win32/msi/view-execute)
supports `?` parameter markers and a Record containing replacement values in
the same order and types. [`Installer.CreateRecord`](https://learn.microsoft.com/en-us/windows/win32/msi/installer-createrecord)
creates that parameter record. The official
[`SQL Syntax`](https://learn.microsoft.com/en-us/windows/win32/msi/sql-syntax)
reference documents Windows Installer's restricted SQL and states that a quote
cannot be escaped inside a string literal. Parameterization is therefore a
correctness requirement as well as an injection defense.

Parameters replace values, not table/column identifiers. The query builder must
accept identifiers only from code-owned descriptors or a closed allowlist. For
example, the two sequence tables may be represented by an enum-like allowlist,
not a free string; a Feature key read from the MSI is a string parameter even if
it also passes the MSI Identifier grammar.

#### COM lifetime

The official [Database object](https://learn.microsoft.com/en-us/windows/win32/msi/database-object)
example uses this order: open database, open view, create/fill a parameter
Record, execute, fetch Records, close the view, and commit only for a modifying
database. [`View.Close`](https://learn.microsoft.com/en-us/windows/win32/msi/view-close)
terminates the query and releases database resources; it must occur before a
view is re-executed unless every row was fetched. [`View.Fetch`](https://learn.microsoft.com/en-us/windows/win32/msi/view-fetch)
returns a Record or null and expects the record reference to be released when it
is no longer needed.

.NET's
[`Marshal.ReleaseComObject`](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.marshal.releasecomobject?view=net-10.0)
and
[`Marshal.FinalReleaseComObject`](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.marshal.finalreleasecomobject?view=net-10.0)
documentation warns that releasing an RCW still in use can cause
`InvalidComObjectException`, access violations, or process corruption,
especially for shared/singleton RCWs. `FinalReleaseComObject` is equivalent to
releasing until the RCW reference count reaches zero.

The safe project rule is consequently ownership, not indiscriminate release:

1. The module creates every installer/database/view/parameter/metadata/fetched
   Record RCW and never returns or shares one.
2. Per query it copies one fetched row to primitives, releases that owned Record,
   and clears the reference before fetching the next.
3. In reverse acquisition order it releases the column-info and parameter
   Records, calls `View.Close`, then releases the View.
4. Session shutdown releases the database and installer last.
5. `Close`/release is attempted on all paths. A cleanup-only failure fails the
   verifier. If a primary query/package failure already exists, preserve that
   failure and attach cleanup details rather than replacing it.
6. Do not call `GC.Collect()`/`WaitForPendingFinalizers()` as normal lifecycle
   control. Prove deterministic closure with an immediate file move/delete and
   repeated-query fixture instead.

### Engineering option comparison

| Option                                                       | Benefits                                                                                                                                                 | Failure modes / limits                                                                                                                                                       | Decision                                                                                                                                                   |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep direct `$record.FieldCount` property                    | Matches Microsoft's documented Automation syntax; detects shape if projection works.                                                                     | Observed as zero on both release runners; exact cause unknown; makes production acceptance depend on a metadata member not otherwise needed.                                 | Reject as a production dependency. Retain only as optional telemetry in a native diagnostic.                                                               |
| Change to `$record.FieldCount()` or reflection `GetProperty` | May distinguish PowerShell binder paths in an experiment.                                                                                                | Parentheses contradict the documented property shape; no authoritative source found that establishes this as the fix; reflection preserves the same COM metadata dependency. | Diagnostic only, never a fallback or acceptance branch.                                                                                                    |
| Pass a bare expected integer column count                    | Small diff; avoids `FieldCount`.                                                                                                                         | Query and count can drift; all-string reads mask type/null errors; out-of-range `StringData` can become empty; arbitrary SQL can return a different list.                    | Insufficient alone. A transitional patch at most.                                                                                                          |
| Explicit ordered result schema                               | Binds names, order, MSI types, nullability, and per-field limits; enables exact ordinal access and useful diagnostics.                                   | Still fragile if raw SQL constructs its own result list independently.                                                                                                       | Required, with schema-driven SELECT construction and `ColumnInfo` validation.                                                                              |
| Parameterized query abstraction                              | Removes runtime values from SQL; handles apostrophes; centralizes identifier allowlists, cardinality, schema, and limits.                                | `?` cannot represent identifiers; abstraction must not grow a permissive raw-SQL escape hatch.                                                                               | Recommended core.                                                                                                                                          |
| Typed primitive-copy helper                                  | Prevents COM escape, preserves null/empty/type distinctions, bounds data before allocation, simplifies callers.                                          | PowerShell output enumeration can distort arrays; classes have module import semantics; careless casts can reintroduce coercion.                                             | Recommended. Return fixed primitive row objects/arrays deliberately and suppress incidental method output.                                                 |
| Extract a `.psm1` module                                     | One ownership boundary, reusable by production and native fixture, source-level API can be statically constrained.                                       | Extraction alone does not fix shape/type/parameter/lifetime behavior; public classes require `using module` for type literals.                                               | Recommended with a function API and module-private session/type implementation.                                                                            |
| Native temporary-MSI integration fixture                     | Exercises the actual `pwsh` + Windows Installer Automation stack cheaply; can prove locks are released and failure paths are closed.                     | Must run on native Windows; fixture construction and production reader must be separate phases; hosted ARM availability can fail.                                            | Required on x64 and ARM64 PR CI.                                                                                                                           |
| Replace Automation with native MSI P/Invoke                  | Native APIs such as `MsiRecordGetFieldCount`, `MsiRecordIsNull`, and typed record reads have explicit handle contracts and avoid dynamic COM projection. | Larger rewrite; introduces handle, buffer, encoding, and P/Invoke safety obligations; not needed if the rest of Automation is stable under the native fixture.               | Reserve as a deliberate future replacement if native tests expose additional Automation projection instability; do not mix it in as a one-member fallback. |

### Recommended query/module design

#### 1. One authoritative query specification

Each query specification should contain, at minimum:

```text
Table                  code-owned MSI identifier
Columns[]              ordered { Name, Kind, Nullable, MaxSize }
Predicate              code-owned SQL fragment containing only ? value markers
Parameters[]           ordered { Kind, Value }
Cardinality            zero-or-one | exactly-one | many
MaxRows                explicit positive limit
MaxAggregateUnits      explicit positive query-wide limit
```

`Kind` should initially be only `String` or `Int32`, because those are the
primitive table fields this verifier consumes. Stream data belongs to a separate
bounded operation. `MaxSize` applies to string UTF-16 units and must be checked
through `DataSize` before `StringData`. `Nullable=false` rejects null before any
conversion. For nullable strings, the returned value is actual `$null`; empty
string remains distinguishable.

The builder must:

- reject zero columns, more than the project limit (currently 16), duplicates,
  invalid MSI identifiers, `SELECT *`, unsupported expression columns, and an
  empty/nonpositive resource budget;
- build the quoted `SELECT` list itself from `Columns` rather than testing the
  prefix of separately supplied SQL;
- choose `Table` and any dynamic column/table identifiers only from the query
  descriptor/closed allowlist;
- require one parameter descriptor for every `?`, create the same-sized Record,
  and set each field with its typed Automation setter;
- never interpolate a parameter value into the query string, including values
  that were read from the same MSI and values that already pass an identifier
  regex;
- open the database in read-only mode `0` and expose no modifying operation.

The result-list construction gives an exact column count by construction.
After `OpenView`, request both `ColumnInfo(0)` and `ColumnInfo(1)` and compare
known ordinals with the expected name and column-definition type. Empty or
mismatched metadata at any expected ordinal rejects the candidate. Extra result
columns are impossible through the non-public builder; do not expose a raw SQL
API that can bypass this invariant.

#### 2. Typed copy and cardinality operations

For each fetched row and each known ordinal:

1. Ask `IsNull(i)`. Reject null when the descriptor is non-nullable; otherwise
   store `$null` and do not call a value accessor.
2. Ask `DataSize(i)` and validate it against the descriptor and remaining
   query-wide budget before materializing a string.
3. Use `StringData(i)` for `String`; use `IntegerData(i)` for `Int32`.
4. Copy the primitive value into a newly allocated result row. The result row
   may be a fixed `PSCustomObject`/ordered map plus an `object[]`, provided every
   element is recursively restricted to approved primitive types and no COM
   object can appear.

The public module API should expose distinct operations rather than force every
caller to rediscover cardinality rules:

- `Invoke-MsiQuery` for a bounded many-row result;
- `Get-MsiOptionalRow` for exactly zero or one, implemented with a two-row cap
  and duplicate rejection;
- `Get-MsiRequiredRow` for exactly one;
- `Export-MsiBoundedStream` for one non-null stream with a caller-supplied total
  byte ceiling and fixed output semantics.

PowerShell functions write all uncaptured expression and method output to the
pipeline. The implementation must use `[void]`/`Out-Null` for COM method return
values and deliberately emit one row collection shape. This follows official
[`about_Return`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_return)
behavior and prevents an `Execute`, `Close`, or collection method result from
becoming a phantom row.

#### 3. Module boundary

Recommended new module:

`scripts/release/WindowsInstallerQuery.psm1`

It should use `Export-ModuleMember` to expose only construction/session/query
functions. A module-private session implementation owns the installer and
database RCWs; callers receive an opaque session value but cannot obtain an RCW.
Query specs and row results can be validated fixed `PSCustomObject` structures
containing primitives. Avoid making a PowerShell class name part of the public
interface: official
[`about_Classes`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_classes)
notes that consumers need `using module` to use classes defined in a module,
whereas `Import-Module` alone does not import the class type for type literals.
The official [script module guidance](https://learn.microsoft.com/en-us/powershell/scripting/developer/module/how-to-write-a-powershell-script-module)
supports `.psm1` plus explicit exports without a new dependency.

`verify-windows-msi-structure.ps1` should import this module, define its
code-owned query specs, and operate only on copied row data. All direct
`OpenView`, `Execute`, `Fetch`, `ColumnInfo`, Record field access, and
`FinalReleaseComObject` calls should disappear from the verifier. Stream export
should use the module's separate bounded stream operation, but cabinet
extraction, PE/hash/signature comparison, and business assertions remain in the
verifier.

### Threat and fail-closed analysis

Treat the candidate MSI as untrusted structured input even though it was built
earlier in the same job. The verifier is a release authority: a false accept can
publish a package with missing security actions or mismatched payload, while a
resource leak can obscure or destabilize later gates.

| Threat / fault                                       | Current-risk pattern                                                                                        | Required defense and closed outcome                                                                                                                                                                                                                                                                                                                          |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Query/value injection or quote-induced query failure | Runtime values are interpolated into restricted MSI SQL; an apostrophe cannot be escaped in a literal.      | Use `?` plus typed parameter Record for all values. Identifiers are compile-time/closed-allowlist only. Any mismatch in marker count/type fails before `OpenView`.                                                                                                                                                                                           |
| Result-shape confusion                               | A metadata count can be zero; a separate expected integer can drift from raw SQL.                           | Build the `SELECT` list from exact descriptors, validate expected `ColumnInfo` ordinals, and expose no raw query bypass. Any name/type mismatch fails.                                                                                                                                                                                                       |
| Missing column accepted as empty                     | `StringData` documents empty for a nonexistent field.                                                       | Never “fall back” from a zero FieldCount. Validate construction/metadata and use exact known ordinals.                                                                                                                                                                                                                                                       |
| Null/empty/zero confusion                            | Universal string reads and downstream casts collapse semantic states.                                       | `IsNull` first; exact typed accessor; declared nullability. No implicit cast is an acceptance test.                                                                                                                                                                                                                                                          |
| Integer/string type confusion                        | `StringData` converts integer/object fields.                                                                | Validate MSI column type and use `IntegerData` only for declared integers. Reject mismatch or null sentinel.                                                                                                                                                                                                                                                 |
| Oversized field or aggregate table                   | String is allocated before the current 1,024-unit check; row cap alone still permits large aggregate input. | `DataSize` before materialization, per-field cap, row cap, column cap, and query-wide aggregate-unit budget. Any arithmetic must be checked for overflow.                                                                                                                                                                                                    |
| Oversized embedded cabinet                           | Any positive stream length is accepted and written.                                                         | Require an explicit positive `MaxStreamBytes` policy before opening output, compare `DataSize` first, read fixed chunks, track exact remaining bytes, reject early/extra data, and delete the partial file. The policy value must be code-owned, versioned, and large enough for reviewed FyAgent artifacts, not derived from MSI-controlled metadata alone. |
| Duplicate rows weaken uniqueness assertions          | A nominal scalar query may take the first match.                                                            | Optional/required-row helpers fetch at most two and reject the second; many-row queries enforce their exact cap.                                                                                                                                                                                                                                             |
| COM RCW escapes or file remains locked               | Raw COM objects and cleanup are interleaved with business assertions.                                       | Module copies primitives only, owns every RCW, closes/release in reverse order, and native tests immediately move/delete the MSI after success and induced failure.                                                                                                                                                                                          |
| Cleanup failure hidden                               | Current `View.Close` failures emit only warnings.                                                           | Cleanup-only failure rejects. On a primary failure, preserve the original exception and attach cleanup diagnostics; never report success.                                                                                                                                                                                                                    |
| Read-only verifier mutates or executes candidate     | A broad helper could expose Commit/install/custom-action paths.                                             | `OpenDatabase(..., 0)` only; production module exports no `Commit`, `Import`, `msiexec`, custom-action, or write operation. Fixture creation is a test-only phase on a new temporary database.                                                                                                                                                               |
| Temporary-file traversal/reparse                     | Candidate-controlled stream/file names can affect extraction.                                               | Keep the current fresh root, fixed output name, non-reparse/exact-child checks, and fixed File key extraction. Parameterization complements rather than replaces filesystem admission.                                                                                                                                                                       |
| Hosted-image drift                                   | PowerShell/COM projection can change without a repository diff.                                             | Required native fixture on both release architectures plus runtime fact logging. No runner fallback or cross-build substitutes a failed architecture.                                                                                                                                                                                                        |

### Native integration fixture: evidence without a local cross-OS path

Microsoft documents
[`Installer.OpenDatabase`](https://learn.microsoft.com/en-us/windows/win32/msi/installer-opendatabase)
mode `3` as create/transact, and
[`Database.Import`](https://learn.microsoft.com/en-us/windows/win32/msi/database-import)
as importing a text archive table. The official
[`Import Files`](https://learn.microsoft.com/en-us/windows/win32/msi/import-files)
sample creates/imports/commits tables, and the
[`Archive File Format`](https://learn.microsoft.com/en-us/windows/win32/msi/archive-file-format)
defines reviewable tab-delimited `.idt` source. Use those contracts to avoid
committing an opaque binary MSI fixture.

Recommended fixture source:

`tests/fixtures/windows-installer-query/QueryFixture.idt`

It should define a test-only table containing:

- a one-column string row matching the failed query shape;
- a multi-column row with string, 2-/4-byte integer, nullable string, empty
  string, and an apostrophe-containing value;
- several rows for ordering/cardinality/row-cap tests;
- a string column suitable for a value just below and just above the project
  limit.

Recommended native test:

`tests/windowsInstallerQuery.integration.ps1`

The test runs only on native Windows. In a fresh exact temporary directory it:

1. Creates a temporary MSI with `OpenDatabase(path, 3)`.
2. Imports the checked-in `.idt`, commits the **test database only**, releases
   all fixture-construction RCWs, and reopens the file read-only through the
   production module.
3. Exercises both success and intentionally failing cases.
4. Closes the session and immediately renames/deletes the MSI and fixture output
   after both paths, proving no file-locking RCW remains.
5. Repeats the query/session cycle enough times to reveal steadily leaked
   handles or RCWs without turning PR CI into a stress benchmark.

Minimum assertions:

- one-column and multi-column result name/order/type;
- `null` distinct from empty string and numeric zero;
- a parameter containing `'` round-trips without entering SQL text;
- integer is returned as `Int32`, not a numeric-looking string;
- zero/one/two rows exercise optional, required, duplicate, and many-row
  semantics;
- wrong expected column name/type/nullability fails;
- wrong parameter count/type fails before query execution;
- row cap, per-field `DataSize` cap, aggregate budget, and stream-byte cap fail
  before uncontrolled allocation/output;
- success results contain no COM object recursively;
- induced query/schema/limit failure preserves its primary diagnostic even if
  cleanup is also exercised;
- immediate MSI move/delete succeeds after success, failure, and repeated
  cycles.

The test should record, in its normal header, the exact
`$PSVersionTable.PSVersion`, OS build, process/runner architecture,
`RUNNER_IMAGE`/image-version context when available, and the file/product version
of `%SystemRoot%\System32\msi.dll`. These facts were absent from the failed-run
log and are necessary to identify future hosted-image drift. A diagnostic may
also record direct-versus-reflection `FieldCount` results, but no pass/fail
assertion or production branch may depend on either value.

No Linux/local invocation of this `.ps1`, copied Windows binaries, WSL bridge,
emulation, or staged MSI is acceptable evidence. This follows the task PRD and
Windows boundary (`prd.md:33-36`; `windows-release-boundary.md:90-96`).

### What belongs in regular PR CI

The current regular Windows job is x64-only and runs Rust check/Clippy/tests
without Windows Installer Automation (`.github/workflows/ci.yml:230-271`). The
only current runtime call occurs after each release job has built the complete
application, installer-actions DLL, and MSI (`.github/workflows/release.yml:429-538`).
That topology explains why a shared query-reader failure was discovered only by
full preflight.

Add one unconditional required job ID, recommended
`windows-msi-query`, with a two-entry native matrix:

```text
windows-2022   / X64
windows-11-arm / ARM64
```

The job needs only pinned checkout and the repository integration script; it
does not install frontend/Rust dependencies or build FyAgent. Both matrix legs
run the same production module and test-generated MSI under the same `pwsh`
family used by Release. The job must have read-only contents permission, no
secrets, no path filter, `fail-fast: false`, and no opposite-architecture
fallback. Its single job ID is added to `CI / Required`'s exact `needs` set and
`scripts/ci/required-gate.mjs`. A failed, cancelled, skipped, missing, or unknown
result remains rejecting.

Why both architectures are required:

- The supported Release boundary explicitly covers both native x64 and ARM64.
- The observed failure happened on both, demonstrating that the abstraction is
  shared.
- x64-only PR coverage would likely catch this exact incident but would still
  defer an ARM-only runner/COM regression to full preflight.

GitHub's current [hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
lists `windows-11-arm` as an ARM64 public-preview runner. Its availability/SLA
is therefore a real CI-operability caveat. Under this repository's fail-closed
release model, unavailability should be a retryable infrastructure failure, not
permission to mark ARM covered by x64 or to cross-build. If maintainers make an
explicit cost/availability decision against required ARM PR coverage, the
weaker fallback is required x64 coverage plus scheduled/manual ARM smoke; that
fallback does **not** fully meet the stated goal of preventing ARM-only discovery
in a later full preflight and is not recommended here.

Keep the full real-bundle structure verifier in both Release matrix legs. The
small fixture proves the query/interop layer; it does not prove Tauri/WiX output,
FyAgent table contents, cabinet binding, PE architecture, hashing, Authenticode,
or installer lifecycle.

### Explicit implementation, test, and spec plan

The following is a recommended future write set; this research made none of
these implementation/spec/test changes.

1. **Add `scripts/release/WindowsInstallerQuery.psm1`.**
   - Implement schema-driven `SELECT` construction, identifier allowlists,
     typed parameter Records, `ColumnInfo` checks, typed/null-preserving copy,
     per-field/row/aggregate limits, optional/required/many cardinality, bounded
     stream export, and exclusive COM ownership.
   - Export only the narrow function API. No raw `OpenView`, modifying database
     operation, or public RCW.
2. **Refactor `scripts/release/verify-windows-msi-structure.ps1`.**
   - Import the module.
   - Replace every interpolated raw query with a query specification and typed
     parameters.
   - Make every caller declare names, MSI types, nullability, field limit,
     cardinality, and row/aggregate budget.
   - Preserve package-specific business assertions and filesystem/cabinet/PE/
     hash/signature checks.
   - Set an explicit reviewed cabinet byte ceiling and make cleanup failures
     fail closed.
3. **Add native source fixture and test.**
   - `tests/fixtures/windows-installer-query/QueryFixture.idt`
   - `tests/windowsInstallerQuery.integration.ps1`
   - Generate the MSI only in the native test temp directory; do not commit a
     binary MSI and never run `msiexec` or a custom action.
4. **Add the required native PR job.**
   - Update `.github/workflows/ci.yml` with the unconditional two-architecture
     `windows-msi-query` matrix and add it to `required.needs`.
   - Update `scripts/ci/required-gate.mjs` from six to seven exact dependency
     IDs.
   - Update `tests/ciWorkflow.test.ts` and `tests/requiredCiGate.test.ts` for the
     new exact topology, runner matrix, native script invocation, permissions,
     no-path-filter rule, and fail/cancel/skip/missing handling.
5. **Strengthen static release contracts.**
   - Extend `tests/releaseWorkflow.test.ts` to prove the verifier imports the
     module; all result columns are schema-defined; runtime values are
     parameters; string/int/null access is typed; `DataSize` precedes
     materialization; all queries/streams carry resource budgets; no
     `.FieldCount`, raw `OpenView`, `Commit`, or COM object remains in the
     verifier; cleanup errors cannot become success.
   - Static checks supplement but never replace the native fixture.
6. **Update executable specs.**
   - `.trellis/spec/backend/windows-release-boundary.md`: replace the current
     incident-specific “explicit Columns/no FieldCount” wording with the durable
     schema/parameter/type/null/resource/COM-ownership contract; require native
     fixture evidence and keep real-bundle Release checks.
   - `.trellis/spec/backend/github-ci-workflow.md`: change the exact Required
     topology to seven job IDs, add the native MSI matrix/runners, and specify
     fail-closed/public-preview behavior.
   - `.trellis/spec/backend/github-release-workflow.md`: state that both Release
     Windows legs consume the same tested module and still validate the real
     candidate MSI.
7. **Validation sequence.**
   - On the non-Windows development host, run only repository static/format/
     TypeScript contract checks allowed by the task; do not claim COM evidence.
   - Let the ordinary PR CI produce one native x64 and one native ARM64 fixture
     result for the exact source. Require both through `CI / Required`.
   - Only after that gate is green, an authorized full unsigned preflight can
     establish real-MSI evidence. This research neither queried nor triggered
     Actions.

Definition of done for the query engineering slice:

- no production code reads or branches on `FieldCount`;
- no runtime MSI value is interpolated into SQL;
- every query has exact names/order/types/nullability/cardinality and finite
  field/row/aggregate limits;
- returned values contain no COM object and preserve null/empty/int semantics;
- success and error paths close all owned resources and allow immediate MSI
  deletion;
- both native PR matrix legs pass the generated-fixture suite for the exact
  source;
- both real-bundle Release legs still pass the complete structure/payload
  verifier before upload;
- specs and the exact Required gate encode the new boundary.

### External references, versions, and dates

All references below are primary Microsoft/GitHub documentation or official
upstream repositories. Retrieval date is 2026-08-08. Document “updated” dates
are the dates displayed by Microsoft when available; an online document date is
not a claim about the version installed in the failed runner.

| Reference                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Published contract used                                                                                                                    | Displayed update / version                                                      |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| Microsoft: [Record.FieldCount](https://learn.microsoft.com/en-us/windows/win32/msi/record-fieldcount)                                                                                                                                                                                                                                                                                                                                                                                                             | Read-only Automation property; result-field count.                                                                                         | 2021-03-22                                                                      |
| Microsoft: [Record.StringData](https://learn.microsoft.com/en-us/windows/win32/msi/record-stringdata)                                                                                                                                                                                                                                                                                                                                                                                                             | 1-based access; coercion to string; nonexistent field returns empty.                                                                       | 2021-03-22                                                                      |
| Microsoft: [Record.IsNull](https://learn.microsoft.com/en-us/windows/win32/msi/record-isnull)                                                                                                                                                                                                                                                                                                                                                                                                                     | Indexed null test.                                                                                                                         | 2021-03-22                                                                      |
| Microsoft: [Record.IntegerData](https://learn.microsoft.com/en-us/windows/win32/msi/record-integerdata)                                                                                                                                                                                                                                                                                                                                                                                                           | Typed 32-bit integer access and database-null sentinel behavior.                                                                           | 2023-06-13 on localized reference retrieved during research                     |
| Microsoft: [Record.DataSize](https://learn.microsoft.com/en-us/windows/win32/msi/record-datasize)                                                                                                                                                                                                                                                                                                                                                                                                                 | Pre-read string/stream size, integer size, null size.                                                                                      | 2021-03-22                                                                      |
| Microsoft: [Record object](https://learn.microsoft.com/en-us/windows/win32/msi/record-object)                                                                                                                                                                                                                                                                                                                                                                                                                     | Field 0 is reserved; record fields can contain different primitive/object kinds.                                                           | 2021-03-22                                                                      |
| Microsoft: [View.ColumnInfo](https://learn.microsoft.com/en-us/windows/win32/msi/view-columninfo)                                                                                                                                                                                                                                                                                                                                                                                                                 | Result-column names/types metadata Record.                                                                                                 | 2021-03-22                                                                      |
| Microsoft: [Column Definition Format](https://learn.microsoft.com/en-us/windows/win32/msi/column-definition-format)                                                                                                                                                                                                                                                                                                                                                                                               | MSI string/integer/nullability type encoding; declared string width is not internally enforced.                                            | 2021-07-01                                                                      |
| Microsoft: [View.Execute](https://learn.microsoft.com/en-us/windows/win32/msi/view-execute)                                                                                                                                                                                                                                                                                                                                                                                                                       | `?` parameters supplied by a Record before Fetch.                                                                                          | 2021-03-22                                                                      |
| Microsoft: [Installer.CreateRecord](https://learn.microsoft.com/en-us/windows/win32/msi/installer-createrecord)                                                                                                                                                                                                                                                                                                                                                                                                   | Creates typed parameter/input Records; field 0 reserved.                                                                                   | 2021-03-22                                                                      |
| Microsoft: [SQL Syntax](https://learn.microsoft.com/en-us/windows/win32/msi/sql-syntax)                                                                                                                                                                                                                                                                                                                                                                                                                           | Restricted Windows Installer SQL and literal limitations.                                                                                  | Current online reference, retrieved 2026-08-08                                  |
| Microsoft: [View.Fetch](https://learn.microsoft.com/en-us/windows/win32/msi/view-fetch)                                                                                                                                                                                                                                                                                                                                                                                                                           | Fetch returns Record or null and record reference is released when no longer needed.                                                       | 2025-10-16                                                                      |
| Microsoft: [View.Close](https://learn.microsoft.com/en-us/windows/win32/msi/view-close)                                                                                                                                                                                                                                                                                                                                                                                                                           | Terminates query and releases database resources.                                                                                          | 2021-03-22                                                                      |
| Microsoft: [Database object](https://learn.microsoft.com/en-us/windows/win32/msi/database-object)                                                                                                                                                                                                                                                                                                                                                                                                                 | Official open-view/parameter/execute/fetch/close ordering.                                                                                 | 2021-03-22                                                                      |
| Microsoft: [Installer.OpenDatabase](https://learn.microsoft.com/en-us/windows/win32/msi/installer-opendatabase)                                                                                                                                                                                                                                                                                                                                                                                                   | Read-only mode 0 and create/transact mode 3.                                                                                               | 2021-04-14                                                                      |
| Microsoft: [Database.Import](https://learn.microsoft.com/en-us/windows/win32/msi/database-import), [Import Files sample](https://learn.microsoft.com/en-us/windows/win32/msi/import-files), [Archive File Format](https://learn.microsoft.com/en-us/windows/win32/msi/archive-file-format)                                                                                                                                                                                                                        | Native temporary MSI fixture from reviewable text table source.                                                                            | Import 2021-03-22; sample 2021-01-07; archive reference current online          |
| Microsoft: [Working with Records](https://learn.microsoft.com/en-us/windows/win32/msi/working-with-records)                                                                                                                                                                                                                                                                                                                                                                                                       | Native MSI record count/null/data-size/typed-read family; supports future full native adapter evaluation.                                  | 2021-01-07                                                                      |
| Microsoft official PowerShell example: [MIM 2016 upgrade](https://learn.microsoft.com/en-us/microsoft-identity-manager/microsoft-identity-manager-2016-upgrade-from-service-pack-2-to-service-pack-3)                                                                                                                                                                                                                                                                                                             | Uses `WindowsInstaller.Installer`, opens via reflection, and reads a known ordinal with `StringData(1)` without a `FieldCount` dependency. | Current 2026 online page, retrieved 2026-08-08                                  |
| Microsoft PowerShell: [about_Methods](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_methods), [about_Return](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_return), [about_Classes](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_classes), [script modules](https://learn.microsoft.com/en-us/powershell/scripting/developer/module/how-to-write-a-powershell-script-module) | Parentheses mean method call; pipeline output unrolls; class import/module API constraints.                                                | `about_Return` 2026-01-18; other current online references retrieved 2026-08-08 |
| Microsoft .NET 10: [ReleaseComObject](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.marshal.releasecomobject?view=net-10.0), [FinalReleaseComObject](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.marshal.finalreleasecomobject?view=net-10.0)                                                                                                                                                                                                       | RCW release semantics and shared-RCW hazards.                                                                                              | .NET 10 reference retrieved 2026-08-08                                          |
| GitHub: [Hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)                                                                                                                                                                                                                                                                                                                                                                                                    | Current labels/architectures; Windows ARM64 is public preview.                                                                             | Live docs retrieved 2026-08-08                                                  |
| GitHub official upstream: [actions/runner-images releases](https://github.com/actions/runner-images/releases)                                                                                                                                                                                                                                                                                                                                                                                                     | Hosted image runtimes evolve, including PowerShell updates; exact runtime facts must be logged per run.                                    | Live upstream releases retrieved 2026-08-08                                     |

### Related specs

- `.trellis/spec/backend/windows-release-boundary.md:12-14` makes the Windows
  release boundary native on both x64 and ARM64; `:90-96` excludes local foreign
  evidence; `:343-349` already says static checks do not replace native real-MSI
  execution.
- `.trellis/spec/backend/github-release-workflow.md:133-152` defines the same
  native runner matrix; `:182-230` requires the Windows structure/payload gates;
  `:353-369` separates host-local checks from native Release evidence.
- `.trellis/spec/backend/github-ci-workflow.md:45-72` currently makes six job IDs
  exact; `:97-110` rejects missing/skipped/weaker paths; `:114-145` currently
  lists only x64 Windows required CI; `:268-295` names the static CI contracts.
  These clauses must be updated atomically with a new required native MSI job.
- `.trellis/tasks/08-07-modernize-ci-and-release/prd.md:33-36` prohibits a local
  cross-OS/cross-architecture substitute, and `:43-45` keeps the real unsigned
  full matrix as acceptance. The lightweight PR fixture advances failure
  detection but does not satisfy that release acceptance criterion.

## Caveats / Not Found

- No authoritative Microsoft, PowerShell, GitHub runner, or other upstream issue
  was found that reproduces `WindowsInstaller.Record.FieldCount == 0` for this
  exact one-column query under PowerShell 7 on both x64 and ARM64. The exact
  interop root cause remains unproven. This is why the recommendation removes
  the unstable metadata dependency through a stronger query contract instead
  of claiming a syntax fix.
- The failed log shows the shell executable under `C:\Program Files\PowerShell\7\pwsh.EXE`
  and SDK tool version `10.0.26100.0`, but does not record the PowerShell patch
  version, runner image version, OS build, COM type library registration, or
  `msi.dll` file/product version. A native fixture must record them.
- The interrupted working-tree explicit-`Columns` implementation and its static
  tests have not been executed on a native Windows runner in the supplied
  evidence. They must not be described as fixing the preflight failure.
- This research ran no PowerShell, Windows executable, foreign binary,
  cross-target, emulator, or local staged-MSI verification. It did not query,
  rerun, or trigger GitHub Actions.
- `windows-11-arm` is currently documented as public preview. A required ARM64
  PR leg improves correctness coverage but adds real availability/cost latency;
  the repository must keep treating unavailable native ARM as retryable
  infrastructure failure, not as authorization for a fallback architecture.
- A native P/Invoke rewrite could eliminate Automation projection entirely, but
  it would add handle/buffer/encoding safety work and is not justified by this
  one observed metadata member alone. Reconsider it only if the proposed native
  fixture demonstrates broader Automation instability.
- The same preflight also recorded separate Linux metadata failures. They are
  outside this Windows MSI query research and do not change the Windows design.
