import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const MODULE_PATH = path.join(
  ROOT,
  "scripts",
  "release",
  "WindowsInstallerQuery.psm1",
);
const VERIFIER_PATH = path.join(
  ROOT,
  "scripts",
  "release",
  "verify-windows-msi-structure.ps1",
);
const INTEGRATION_PATH = path.join(
  ROOT,
  "tests",
  "windowsInstallerQuery.integration.ps1",
);
const FIXTURE_ROOT = path.join(
  ROOT,
  "tests",
  "fixtures",
  "windows-installer-query",
);

const moduleSource = fs
  .readFileSync(MODULE_PATH, "utf8")
  .replace(/\r\n/g, "\n");
const verifierSource = fs
  .readFileSync(VERIFIER_PATH, "utf8")
  .replace(/\r\n/g, "\n");
const integrationSource = fs
  .readFileSync(INTEGRATION_PATH, "utf8")
  .replace(/\r\n/g, "\n");

function functionBlock(source: string, name: string): string {
  const match = new RegExp(`^\\s*function ${name}(?: \\{|\\()`, "m").exec(
    source,
  );
  const start = match?.index ?? -1;
  expect(start, name).toBeGreaterThanOrEqual(0);
  const afterStart = start + (match?.[0].length ?? 0);
  const next = source
    .slice(afterStart)
    .search(/^\s*function [A-Za-z0-9-]+(?: \{|\()/m);
  return next < 0
    ? source.slice(start)
    : source.slice(start, afterStart + next);
}

describe("Windows Installer query boundary", () => {
  it("exports only the opaque-session, typed-row, summary, and bounded-stream API", () => {
    const exported = [
      ...moduleSource.matchAll(/^\s+'([A-Za-z][A-Za-z0-9-]+)'[,]?$/gm),
    ].map((match) => match[1]);
    expect(exported.slice(-7)).toEqual([
      "Open-MsiQuerySession",
      "Close-MsiQuerySession",
      "Invoke-MsiQuery",
      "Get-MsiOptionalRow",
      "Get-MsiRequiredRow",
      "Export-MsiBoundedStream",
      "Get-MsiSummaryString",
    ]);
    expect(moduleSource).toContain("$script:MsiQuerySessions");
    expect(moduleSource).toContain("return $sessionId");
    expect(moduleSource).not.toMatch(
      /Export-ModuleMember[^]*?(?:OpenView|Execute|Fetch|Commit)/,
    );
  });

  it("constructs projections from a closed identifier registry and binds every value", () => {
    const builder = functionBlock(moduleSource, "New-MsiSelectCommand");
    const parameters = functionBlock(moduleSource, "New-MsiParameterRecord");

    expect(moduleSource).toContain("$script:MsiQueryColumns = @{");
    for (const table of [
      "Binary",
      "Component",
      "ControlEvent",
      "CustomAction",
      "Directory",
      "File",
      "InstallExecuteSequence",
      "InstallUISequence",
      "Media",
      "Property",
      "_Streams",
    ]) {
      expect(moduleSource, table).toMatch(new RegExp(`^  ${table} = `, "m"));
    }
    expect(builder).toContain(
      "\"SELECT $($quotedColumns -join ', ') FROM ``$Table``\"",
    );
    expect(builder).toContain('"``$($_.Column)`` = ?"');
    expect(builder).not.toContain("$filter.Value");
    expect(parameters).toContain("$Installer.CreateRecord($Filters.Count)");
    expect(parameters).toContain("$record.StringData($index) =");
    expect(parameters).toContain("$record.IntegerData($index) =");
    expect(moduleSource).toContain("MSI query filter type mismatch");
    expect(moduleSource).not.toMatch(/SELECT\s+\*/i);
    expect(verifierSource).not.toMatch(/\bSELECT\b/);
    expect(verifierSource).not.toContain("-Query");
  });

  it("validates known metadata ordinals and materializes exact primitive types", () => {
    const metadata = functionBlock(
      moduleSource,
      "Assert-MsiProjectionMetadata",
    );
    const copy = functionBlock(moduleSource, "Copy-MsiRecordValues");

    expect(moduleSource).toContain("$view.ColumnInfo(0)");
    expect(moduleSource).toContain("$view.ColumnInfo(1)");
    expect(moduleSource).not.toContain(".FieldCount");
    expect(verifierSource).not.toContain(".FieldCount");
    expect(metadata).toContain("$NameRecord.IsNull($index)");
    expect(metadata).toContain("$NameRecord.DataSize($index)");
    expect(metadata).toContain("$NameRecord.StringData($index)");
    expect(metadata).toContain("$TypeRecord.IsNull($index)");
    expect(metadata).toContain("$TypeRecord.DataSize($index)");
    expect(metadata).toContain("$TypeRecord.StringData($index)");
    expect(metadata).toContain("$actualName.Length -ne $nameSize");
    expect(metadata).toContain("$actualType.Length -ne $typeSize");
    expect(metadata.indexOf(".DataSize(")).toBeLessThan(
      metadata.indexOf(".StringData("),
    );

    expect(copy).toContain("$Record.IsNull($index)");
    expect(copy).toContain("$Record.DataSize($index)");
    expect(copy).toContain("$Record.StringData($index)");
    expect(copy).toContain("$Record.IntegerData($index)");
    expect(copy).toContain("$stringValue.Length -ne $dataSize");
    expect(copy.indexOf("$Record.IsNull($index)")).toBeLessThan(
      copy.indexOf("$Record.DataSize($index)"),
    );
    expect(copy.indexOf("$Record.DataSize($index)")).toBeLessThan(
      copy.indexOf("$Record.StringData($index)"),
    );
    expect(copy).toContain("[object[]]::new($Columns.Count)");
    expect(copy).toContain("[int]$Record.IntegerData($index)");
    expect(moduleSource).toContain("[PSCustomObject]@{ Values = $values }");
  });

  it("enforces finite field, row, cell, unit, cardinality, and stream limits", () => {
    for (const declaration of [
      "$script:MaximumColumnCount = 16",
      "$script:MaximumRowCount = 32768",
      "$script:MaximumFieldUnits = 1MB",
      "$script:MaximumAggregateCells = 524288",
      "$script:MaximumAggregateUnits = 256MB",
      "$script:MaximumStreamBytes = 1GB",
    ]) {
      expect(moduleSource).toContain(declaration);
    }
    expect(moduleSource).toContain("$dataSize -gt $column.MaxSize");
    expect(moduleSource).toContain("cell aggregate cap");
    expect(moduleSource).toContain("unit aggregate cap");
    expect(moduleSource).toContain("row cap $MaxRows");
    expect(functionBlock(moduleSource, "Get-MsiOptionalRow")).toContain(
      "-MaxRows 2",
    );
    expect(functionBlock(moduleSource, "Get-MsiOptionalRow")).toContain(
      "expected at most one row",
    );
    expect(functionBlock(moduleSource, "Get-MsiRequiredRow")).toContain(
      "expected exactly one row",
    );

    const stream = functionBlock(moduleSource, "Export-MsiBoundedStream");
    expect(moduleSource).toContain(
      "_Streams = @{ KeyColumn = 'Name'; StreamColumn = 'Data'; Nullable = $true }",
    );
    expect(moduleSource).toContain(
      "_FyAgentQueryStream = @{ KeyColumn = 'Id'; StreamColumn = 'Payload'; Nullable = $false }",
    );
    expect(moduleSource).toContain("$actualType -cnotmatch '^[vV]0$'");
    expect(moduleSource).toContain("$actualNullable -ne $Nullable");
    expect(stream).toContain("$record.IsNull(1)");
    expect(stream).toContain("$record.DataSize(1)");
    expect(stream.indexOf("$record.DataSize(1)")).toBeLessThan(
      stream.indexOf("[IO.File]::Open("),
    );
    expect(stream).toContain("-gt $MaxStreamBytes");
    expect(stream).toContain("[Math]::Min(1MB, $remaining)");
    expect(stream).toContain("[Text.EncoderFallback]::ExceptionFallback");
    expect(stream).toContain(
      "$createdOutput -and (Test-Path -LiteralPath $resolvedOutput)",
    );
    expect(stream).toContain("partial stream output removal");
  });

  it("owns COM deterministically and makes cleanup-only errors rejecting", () => {
    for (const helper of [
      "Add-MsiCleanupError",
      "Release-MsiComObject",
      "Throw-MsiOperationFailure",
    ]) {
      expect(functionBlock(moduleSource, helper), helper).toContain(
        "[AllowEmptyCollection()]",
      );
    }
    expect(moduleSource).toContain("OpenDatabase($resolvedPath, 0)");
    expect(moduleSource).toContain("FinalReleaseComObject($Value)");
    expect(moduleSource).toContain("[void]$view.Close()");
    expect(moduleSource).toContain("cleanup failed:");
    expect(moduleSource).toContain("$PrimaryError");
    expect(moduleSource).not.toMatch(
      /\[GC\]::(?:Collect|WaitForPendingFinalizers)/,
    );
    expect(moduleSource).not.toContain("Write-Warning");

    expect(verifierSource).toContain(
      "Import-Module -Name $queryModule -Force -ErrorAction Stop",
    );
    expect(verifierSource).toContain("Open-MsiQuerySession -Path $resolvedMsi");
    expect(verifierSource).toContain(
      "Close-MsiQuerySession -SessionId $sessionId",
    );
    expect(verifierSource).not.toMatch(
      /WindowsInstaller\.Installer|OpenDatabase|OpenView|\.Execute\(|\.Fetch\(|ColumnInfo|FinalReleaseComObject/,
    );
    expect(verifierSource).not.toMatch(
      /\[GC\]::(?:Collect|WaitForPendingFinalizers)/,
    );
  });

  it("keeps release-specific structure and payload assertions in the verifier", () => {
    for (const contract of [
      "FyAgentInstallerActions",
      "ProductVersion",
      "ApplyValidatedFyAgentInstallDir",
      "AbortUnsafeFyAgentInstallDir",
      "ClassifyFyAgentPureUninstall",
      "MsiLockPermissionsEx",
      "DISABLEADVTSHORTCUTS",
      "FeatureComponents",
      "RemoveShortcuts before RemoveFiles",
      "Get-AuthenticodeSignature",
      "Get-MsiSummaryString",
    ]) {
      expect(verifierSource, contract).toContain(contract);
    }
    expect(verifierSource).toContain("$maxDirectoryRows = 4096");
    expect(verifierSource).toContain("$maxComponentRows = 32768");
    expect(verifierSource).toContain("$maxMsiFieldUtf16Units = 1024");
    expect(verifierSource).toContain("$maxCabinetStreamBytes = 512MB");
    const sequenceReader = functionBlock(verifierSource, "Get-MsiSequenceRow");
    expect(sequenceReader).toContain("$null -eq $sequenceValue");
    expect(sequenceReader).toContain("$sequenceValue -isnot [int]");
    expect(sequenceReader).toContain("$sequenceValue -le 0");
    expect(sequenceReader.indexOf("$null -eq $sequenceValue")).toBeLessThan(
      sequenceReader.indexOf("Sequence = [int]$sequenceValue"),
    );
  });

  it("builds the native MSI only from reviewable IDT schemas and exercises both paths", () => {
    const fixtureFiles = fs.readdirSync(FIXTURE_ROOT).sort();
    expect(fixtureFiles).toEqual([
      "_FyAgentQueryFixture.idt",
      "_FyAgentQueryStream.idt",
      "_FyAgentQueryTypeMismatch.idt",
      "_SummaryInformation.idt",
    ]);
    expect(fixtureFiles.some((name) => /\.msi$/i.test(name))).toBe(false);

    const rows = fs
      .readFileSync(path.join(FIXTURE_ROOT, "_FyAgentQueryFixture.idt"), "utf8")
      .replace(/\r\n/g, "\n");
    expect(rows).toContain(
      "s72\ts72\ts255\ti4\ti2\tS255\ts255\n_FyAgentQueryFixture\tId\n",
    );
    expect(rows).toContain("O'Brien");
    expect(rows.match(/^duplicate-[ab]\tduplicate\t/gm)).toHaveLength(2);

    const streamRows = fs
      .readFileSync(path.join(FIXTURE_ROOT, "_FyAgentQueryStream.idt"), "utf8")
      .replace(/\r\n/g, "\n");
    expect(streamRows).toContain("s72\tv0\n_FyAgentQueryStream\tId\n");
    expect(streamRows).toContain("small\tsmall.ibd");
    expect(streamRows).toContain("large\tlarge.ibd");

    const mismatchRows = fs
      .readFileSync(
        path.join(FIXTURE_ROOT, "_FyAgentQueryTypeMismatch.idt"),
        "utf8",
      )
      .replace(/\r\n/g, "\n");
    expect(mismatchRows).toContain("s72\ti4\n_FyAgentQueryTypeMismatch\tId\n");

    const summaryRows = fs
      .readFileSync(path.join(FIXTURE_ROOT, "_SummaryInformation.idt"), "utf8")
      .replace(/\r\n/g, "\n");
    expect(summaryRows).toContain(
      "i2\tl255\n_SummaryInformation\tPropertyId\n7\tIntel;1033\n",
    );

    expect(integrationSource).toContain(
      "New-Object -ComObject WindowsInstaller.Installer",
    );
    expect(integrationSource).toContain("OpenDatabase($DatabasePath, 3)");
    expect(integrationSource).toContain("$database.Import($ImportRoot, $name)");
    expect(integrationSource).toContain("$database.Commit()");
    expect(integrationSource).toContain("$view.Modify(1, $record)");
    expect(integrationSource).toContain("Import-Module -Name $modulePath");
    expect(integrationSource).toContain("O'Brien");
    expect(integrationSource).toContain(
      "Int32 parameter round-trips through CreateRecord",
    );
    expect(integrationSource).toContain("database null stays null");
    expect(integrationSource).toContain(
      "empty string stays distinct from null",
    );
    expect(integrationSource).toContain("integer is copied as Int32");
    expect(integrationSource).toContain(
      "summary string is copied before its COM owner is released",
    );
    expect(integrationSource).toContain(
      "nullable standard _Streams schema is accepted for a non-null value",
    );
    expect(integrationSource).toContain("at most one row");
    expect(integrationSource).toContain("type mismatch");
    expect(integrationSource).toContain("received i4");
    expect(integrationSource).toContain("nullability mismatch");
    expect(integrationSource).toContain("row cap");
    expect(integrationSource).toContain("cell aggregate cap");
    expect(integrationSource).toContain("unit aggregate cap");
    expect(integrationSource).toContain("-MaxStreamBytes 8");
    expect(integrationSource).toContain("Assert-DatabaseMoveRoundTrip");
    expect(integrationSource).toContain(
      "combined failure preserves its primary diagnostic as the inner exception",
    );
    expect(integrationSource).toContain("for ($cycle = 0; $cycle -lt 8;");
    expect(integrationSource).toContain(
      "Remove-Item -LiteralPath $msiPath -Force -ErrorAction Stop",
    );
    expect(
      functionBlock(integrationSource, "Release-FixtureComObject"),
    ).toContain("[AllowEmptyCollection()]");
    expect(integrationSource).toContain(
      "Assert-Equal -Actual $env:RUNNER_OS -Expected 'Windows'",
    );
    expect(integrationSource).toContain(
      "Assert-Equal -Actual $env:RUNNER_ARCH -Expected $expectedRunnerArchitecture",
    );
    expect(integrationSource).toContain("msiDllFileVersion");
    expect(integrationSource).toContain("runnerImageVersion");
    expect(integrationSource).toContain("osVersion");
    expect(integrationSource).not.toMatch(
      /\[GC\]::(?:Collect|WaitForPendingFinalizers)/,
    );
  });
});
