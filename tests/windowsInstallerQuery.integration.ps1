param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('X64', 'Arm64')]
  [string]$ExpectedArchitecture
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

if (-not $IsWindows) {
  throw 'Windows Installer query integration tests require native Windows'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$modulePath = Join-Path $repositoryRoot 'scripts/release/WindowsInstallerQuery.psm1'
$fixtureSource = Join-Path $PSScriptRoot 'fixtures/windows-installer-query'
$temporaryRoot = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
  [IO.Path]::GetTempPath()
} else {
  $env:RUNNER_TEMP
}
$testRoot = Join-Path $temporaryRoot "fyagent-windows-installer-query-$([Guid]::NewGuid().ToString('N'))"
$fixtureRoot = Join-Path $testRoot 'fixture'
$msiPath = Join-Path $testRoot 'query-fixture.msi'

function Assert-True {
  param(
    [Parameter(Mandatory = $true)][bool]$Condition,
    [Parameter(Mandatory = $true)][string]$Message
  )

  if (-not $Condition) {
    throw "Assertion failed: $Message"
  }
}

function Assert-Equal {
  param(
    [AllowNull()][object]$Actual,
    [AllowNull()][object]$Expected,
    [Parameter(Mandatory = $true)][string]$Message
  )

  if ($null -eq $Actual -or $null -eq $Expected) {
    if ($null -ne $Actual -or $null -ne $Expected) {
      throw "Assertion failed: $Message (actual=$Actual expected=$Expected)"
    }
    return
  }
  if ($Actual -cne $Expected) {
    throw "Assertion failed: $Message (actual=$Actual expected=$Expected)"
  }
}

function Invoke-ExpectedFailure {
  param(
    [Parameter(Mandatory = $true)][scriptblock]$Action,
    [string]$MessageContains = ''
  )

  $caught = $null
  try {
    [void](& $Action)
  } catch {
    $caught = $_.Exception
  }
  if ($null -eq $caught) {
    throw 'Assertion failed: operation unexpectedly succeeded'
  }
  if (
    -not [string]::IsNullOrEmpty($MessageContains) -and
    -not $caught.Message.Contains($MessageContains, [StringComparison]::OrdinalIgnoreCase)
  ) {
    throw "Assertion failed: expected failure containing '$MessageContains', received '$($caught.Message)'"
  }
  return $caught
}

function Assert-NoComObject {
  param(
    [AllowNull()][object]$Value,
    [string]$Path = 'result'
  )

  if ($null -eq $Value) {
    return
  }
  if ([Runtime.InteropServices.Marshal]::IsComObject($Value)) {
    throw "Assertion failed: COM object escaped through $Path"
  }
  if (
    $Value -is [string] -or
    $Value -is [int] -or
    $Value -is [bool]
  ) {
    return
  }
  if ($Value -is [Array]) {
    for ($index = 0; $index -lt $Value.Count; $index += 1) {
      Assert-NoComObject -Value $Value[$index] -Path "$Path[$index]"
    }
    return
  }
  foreach ($property in $Value.PSObject.Properties) {
    Assert-NoComObject -Value $property.Value -Path "$Path.$($property.Name)"
  }
}

function New-StringColumn {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [switch]$Nullable,
    [int]$MaxSize = 255
  )

  return [PSCustomObject]@{
    Name = $Name
    Kind = 'String'
    Nullable = [bool]$Nullable
    MaxSize = $MaxSize
  }
}

function New-IntColumn {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [switch]$Nullable
  )

  return [PSCustomObject]@{
    Name = $Name
    Kind = 'Int32'
    Nullable = [bool]$Nullable
    MaxSize = 4
  }
}

function New-StringFilter {
  param(
    [Parameter(Mandatory = $true)][string]$Column,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value
  )

  return [PSCustomObject]@{ Column = $Column; Kind = 'String'; Value = $Value }
}

function New-IntFilter {
  param(
    [Parameter(Mandatory = $true)][string]$Column,
    [Parameter(Mandatory = $true)][int]$Value
  )

  return [PSCustomObject]@{ Column = $Column; Kind = 'Int32'; Value = $Value }
}

function Release-FixtureComObject {
  param(
    [AllowNull()][object]$Value,
    [Parameter(Mandatory = $true)][string]$Description,
    [Parameter(Mandatory = $true)][Collections.Generic.List[string]]$Errors
  )

  if ($null -eq $Value) {
    return
  }
  try {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Value)
  } catch {
    [void]$Errors.Add("${Description}: $($_.Exception.Message)")
  }
}

function Add-FixtureStandardStream {
  param(
    [Parameter(Mandatory = $true)][object]$Installer,
    [Parameter(Mandatory = $true)][object]$Database,
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][string]$InputPath
  )

  $view = $null
  $record = $null
  $primaryError = $null
  try {
    $view = $Database.OpenView('SELECT `Name`, `Data` FROM `_Streams`')
    [void]$view.Execute()
    $record = $Installer.CreateRecord(2)
    $record.StringData(1) = $Name
    [void]$record.SetStream(2, $InputPath)
    [void]$view.Modify(1, $record)
  } catch {
    $primaryError = $_.Exception
  }

  $cleanupErrors = [Collections.Generic.List[string]]::new()
  Release-FixtureComObject -Value $record -Description 'fixture stream Record' -Errors $cleanupErrors
  $record = $null
  if ($null -ne $view) {
    try {
      [void]$view.Close()
    } catch {
      [void]$cleanupErrors.Add("fixture stream View.Close: $($_.Exception.Message)")
    }
  }
  Release-FixtureComObject -Value $view -Description 'fixture stream View' -Errors $cleanupErrors
  $view = $null
  if ($null -ne $primaryError) {
    if ($cleanupErrors.Count -gt 0) {
      throw [InvalidOperationException]::new(
        "$($primaryError.Message) | fixture stream cleanup failed: $($cleanupErrors -join '; ')",
        $primaryError
      )
    }
    throw $primaryError
  }
  if ($cleanupErrors.Count -gt 0) {
    throw "Fixture stream cleanup failed: $($cleanupErrors -join '; ')"
  }
}

function New-FixtureDatabase {
  param(
    [Parameter(Mandatory = $true)][string]$DatabasePath,
    [Parameter(Mandatory = $true)][string]$ImportRoot
  )

  New-Item -ItemType Directory -Path $ImportRoot -ErrorAction Stop | Out-Null
  $fixtureNames = @(
    '_FyAgentQueryFixture.idt',
    '_FyAgentQueryStream.idt',
    '_FyAgentQueryTypeMismatch.idt',
    '_SummaryInformation.idt'
  )
  foreach ($name in $fixtureNames) {
    $sourcePath = Join-Path $fixtureSource $name
    $destinationPath = Join-Path $ImportRoot $name
    $fixtureText = [IO.File]::ReadAllText($sourcePath)
    $fixtureText = $fixtureText.Replace("`r`n", "`n").Replace("`r", "`n")
    $fixtureText = $fixtureText.Replace("`n", "`r`n")
    [IO.File]::WriteAllText($destinationPath, $fixtureText, [Text.Encoding]::ASCII)
  }
  $streamRoot = Join-Path $ImportRoot '_FyAgentQueryStream'
  New-Item -ItemType Directory -Path $streamRoot -ErrorAction Stop | Out-Null
  [IO.File]::WriteAllBytes(
    (Join-Path $streamRoot 'small.ibd'),
    [byte[]](0, 1, 2, 3, 254, 255)
  )
  [IO.File]::WriteAllBytes(
    (Join-Path $streamRoot 'large.ibd'),
    [byte[]](0..63)
  )

  $installer = $null
  $database = $null
  $primaryError = $null
  try {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase($DatabasePath, 3)
    foreach ($name in $fixtureNames) {
      [void]$database.Import($ImportRoot, $name)
    }
    Add-FixtureStandardStream `
      -Installer $installer `
      -Database $database `
      -Name 'fixture-standard-stream' `
      -InputPath (Join-Path $streamRoot 'small.ibd')
    [void]$database.Commit()
  } catch {
    $primaryError = $_.Exception
  }

  $cleanupErrors = [Collections.Generic.List[string]]::new()
  Release-FixtureComObject -Value $database -Description 'fixture Database' -Errors $cleanupErrors
  Release-FixtureComObject -Value $installer -Description 'fixture Installer' -Errors $cleanupErrors
  if ($null -ne $primaryError) {
    if ($cleanupErrors.Count -gt 0) {
      throw [InvalidOperationException]::new(
        "$($primaryError.Message) | fixture cleanup failed: $($cleanupErrors -join '; ')",
        $primaryError
      )
    }
    throw $primaryError
  }
  if ($cleanupErrors.Count -gt 0) {
    throw "Fixture cleanup failed: $($cleanupErrors -join '; ')"
  }
}

function Invoke-WithQuerySession {
  param(
    [Parameter(Mandatory = $true)][string]$DatabasePath,
    [Parameter(Mandatory = $true)][scriptblock]$Action
  )

  $sessionId = Open-MsiQuerySession -Path $DatabasePath
  $primaryError = $null
  try {
    [void](& $Action $sessionId)
  } catch {
    $primaryError = $_.Exception
  }
  $cleanupError = $null
  try {
    Close-MsiQuerySession -SessionId $sessionId
  } catch {
    $cleanupError = $_.Exception
  }
  if ($null -ne $primaryError) {
    if ($null -ne $cleanupError) {
      throw [InvalidOperationException]::new(
        "$($primaryError.Message) | session cleanup failed: $($cleanupError.Message)",
        $primaryError
      )
    }
    throw $primaryError
  }
  if ($null -ne $cleanupError) {
    throw $cleanupError
  }
}

function Assert-DatabaseMoveRoundTrip {
  param(
    [Parameter(Mandatory = $true)][string]$DatabasePath,
    [Parameter(Mandatory = $true)][string]$Suffix
  )

  $moved = "$DatabasePath.$Suffix"
  Move-Item -LiteralPath $DatabasePath -Destination $moved -ErrorAction Stop
  Move-Item -LiteralPath $moved -Destination $DatabasePath -ErrorAction Stop
}

$actualArchitecture = [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
Assert-Equal -Actual $actualArchitecture -Expected $ExpectedArchitecture -Message 'process architecture matches the native matrix leg'
$expectedRunnerArchitecture = if ($ExpectedArchitecture -ceq 'Arm64') { 'ARM64' } else { 'X64' }
Assert-Equal -Actual $env:RUNNER_OS -Expected 'Windows' -Message 'runner context OS matches native Windows'
Assert-Equal -Actual $env:RUNNER_ARCH -Expected $expectedRunnerArchitecture -Message 'runner context architecture matches the native matrix leg'
$msiDll = Get-Item -LiteralPath (Join-Path $env:SystemRoot 'System32/msi.dll') -ErrorAction Stop
$diagnostics = [ordered]@{
  powershell = $PSVersionTable.PSVersion.ToString()
  os = [Runtime.InteropServices.RuntimeInformation]::OSDescription
  osVersion = [Environment]::OSVersion.VersionString
  processArchitecture = $actualArchitecture
  runnerArchitecture = $env:RUNNER_ARCH
  runnerImage = $env:RUNNER_IMAGE
  runnerImageOs = $env:ImageOS
  runnerImageVersion = $env:ImageVersion
  msiDllFileVersion = $msiDll.VersionInfo.FileVersion
  msiDllProductVersion = $msiDll.VersionInfo.ProductVersion
}
Write-Output "Windows Installer query diagnostics: $($diagnostics | ConvertTo-Json -Compress)"

New-Item -ItemType Directory -Path $testRoot -ErrorAction Stop | Out-Null
$testPrimaryError = $null
$successMessage = $null
try {
  New-FixtureDatabase -DatabasePath $msiPath -ImportRoot $fixtureRoot
  Import-Module -Name $modulePath -Force -ErrorAction Stop

  # Windows Installer's StringData setter maps an empty string to database
  # null. Exercise the distinct non-null/zero-length materialization branch
  # directly with a record-shaped test double while native rows cover the COM
  # null and integer-zero paths below.
  $emptyRecord = [PSCustomObject]@{}
  $emptyRecord | Add-Member -MemberType ScriptMethod -Name IsNull -Value { param($Index) $false }
  $emptyRecord | Add-Member -MemberType ScriptMethod -Name DataSize -Value { param($Index) 0 }
  $emptyRecord | Add-Member -MemberType ScriptMethod -Name StringData -Value { param($Index) '' }
  $queryModule = Get-Module -Name WindowsInstallerQuery -ErrorAction Stop
  $emptyRow = & $queryModule {
    param($Record)

    [int]$aggregateCells = 0
    [long]$aggregateUnits = 0
    Copy-MsiRecordValues `
      -Record $Record `
      -Columns @([PSCustomObject]@{
        Name = 'EmptyValue'
        Kind = 'String'
        Nullable = $false
        MaxSize = 255
      }) `
      -AggregateCells ([ref]$aggregateCells) `
      -AggregateUnits ([ref]$aggregateUnits) `
      -MaxAggregateCells 1 `
      -MaxAggregateUnits 1
  } $emptyRecord
  Assert-Equal -Actual $emptyRow.Values[0] -Expected '' -Message 'empty string stays distinct from null'
  Assert-NoComObject -Value $emptyRow

  [void](Invoke-ExpectedFailure -MessageContains 'cleanup failed' -Action {
    & $queryModule {
      $cleanupErrors = [Collections.Generic.List[string]]::new()
      [void]$cleanupErrors.Add('synthetic cleanup failure')
      Throw-MsiOperationFailure `
        -PrimaryError $null `
        -CleanupErrors $cleanupErrors `
        -Context 'synthetic cleanup-only operation'
    }
  })
  $combinedError = Invoke-ExpectedFailure -MessageContains 'synthetic cleanup failure' -Action {
    & $queryModule {
      $cleanupErrors = [Collections.Generic.List[string]]::new()
      [void]$cleanupErrors.Add('synthetic cleanup failure')
      Throw-MsiOperationFailure `
        -PrimaryError ([InvalidOperationException]::new('synthetic primary failure')) `
        -CleanupErrors $cleanupErrors `
        -Context 'synthetic combined operation'
    }
  }
  Assert-Equal `
    -Actual $combinedError.InnerException.Message `
    -Expected 'synthetic primary failure' `
    -Message 'combined failure preserves its primary diagnostic as the inner exception'

  Invoke-WithQuerySession -DatabasePath $msiPath -Action {
    param($sessionId)

    $oneColumn = Get-MsiRequiredRow `
      -SessionId $sessionId `
      -Table '_FyAgentQueryFixture' `
      -Columns @((New-StringColumn -Name 'Id')) `
      -Filters @((New-StringFilter -Column 'Id' -Value 'single')) `
      -MaxAggregateCells 4 `
      -MaxAggregateUnits 1024
    Assert-Equal -Actual $oneColumn.Values.Count -Expected 1 -Message 'one-column projection has one value'
    Assert-Equal -Actual $oneColumn.Values[0] -Expected 'single' -Message 'one-column projection value'

    $multiColumn = Get-MsiRequiredRow `
      -SessionId $sessionId `
      -Table '_FyAgentQueryFixture' `
      -Columns @(
        (New-StringColumn -Name 'TextValue'),
        (New-IntColumn -Name 'NumberValue'),
        (New-IntColumn -Name 'SmallNumber'),
        (New-StringColumn -Name 'NullableValue' -Nullable),
        (New-StringColumn -Name 'EmptyValue')
      ) `
      -Filters @((New-StringFilter -Column 'Id' -Value 'single')) `
      -MaxAggregateCells 10 `
      -MaxAggregateUnits 4096
    Assert-Equal -Actual $multiColumn.Values[0] -Expected 'one-column' -Message 'string projection'
    Assert-Equal -Actual $multiColumn.Values[1] -Expected 7 -Message 'four-byte integer projection'
    Assert-Equal -Actual $multiColumn.Values[2] -Expected 0 -Message 'two-byte integer zero projection'
    Assert-True -Condition ($multiColumn.Values[1] -is [int]) -Message 'integer is copied as Int32'
    Assert-True -Condition ($multiColumn.Values[2] -is [int]) -Message 'short integer is copied as Int32'
    Assert-True -Condition ($null -eq $multiColumn.Values[3]) -Message 'database null stays null'
    Assert-Equal -Actual $multiColumn.Values[4] -Expected 'EMPTY_PLACEHOLDER' -Message 'final string projection'

    $apostrophe = Get-MsiRequiredRow `
      -SessionId $sessionId `
      -Table '_FyAgentQueryFixture' `
      -Columns @((New-StringColumn -Name 'Id')) `
      -Filters @((New-StringFilter -Column 'TextValue' -Value "O'Brien")) `
      -MaxAggregateCells 4 `
      -MaxAggregateUnits 1024
    Assert-Equal -Actual $apostrophe.Values[0] -Expected 'apostrophe' -Message 'apostrophe parameter round-trips through CreateRecord'

    $integerParameter = Get-MsiRequiredRow `
      -SessionId $sessionId `
      -Table '_FyAgentQueryFixture' `
      -Columns @((New-StringColumn -Name 'Id')) `
      -Filters @((New-IntFilter -Column 'NumberValue' -Value 7)) `
      -MaxAggregateCells 4 `
      -MaxAggregateUnits 1024
    Assert-Equal -Actual $integerParameter.Values[0] -Expected 'single' -Message 'Int32 parameter round-trips through CreateRecord'

    $optional = Get-MsiOptionalRow `
      -SessionId $sessionId `
      -Table '_FyAgentQueryFixture' `
      -Columns @((New-StringColumn -Name 'Id')) `
      -Filters @((New-StringFilter -Column 'Id' -Value 'absent')) `
      -MaxAggregateCells 4 `
      -MaxAggregateUnits 1024
    Assert-True -Condition ($null -eq $optional) -Message 'optional row permits zero matches'

    $many = @(Invoke-MsiQuery `
      -SessionId $sessionId `
      -Table '_FyAgentQueryFixture' `
      -Columns @((New-StringColumn -Name 'Id')) `
      -MaxRows 8 `
      -MaxAggregateCells 8 `
      -MaxAggregateUnits 4096)
    Assert-Equal -Actual $many.Count -Expected 5 -Message 'bounded many-row query returns every fixture row'
    Assert-NoComObject -Value @(
      $oneColumn,
      $multiColumn,
      $apostrophe,
      $integerParameter,
      $many
    )

    $summaryTemplate = Get-MsiSummaryString `
      -SessionId $sessionId `
      -PropertyId 7 `
      -MaxSize 128
    Assert-Equal `
      -Actual $summaryTemplate `
      -Expected 'Intel;1033' `
      -Message 'summary string is copied before its COM owner is released'
    Assert-NoComObject -Value $summaryTemplate

    $streamPath = Join-Path $testRoot 'small-stream.bin'
    $stream = Export-MsiBoundedStream `
      -SessionId $sessionId `
      -Table '_FyAgentQueryStream' `
      -KeyColumn 'Id' `
      -StreamColumn 'Payload' `
      -KeyValue 'small' `
      -OutputPath $streamPath `
      -MaxStreamBytes 16
    Assert-Equal -Actual $stream.SizeBytes -Expected 6 -Message 'bounded stream size'
    Assert-Equal `
      -Actual ([Convert]::ToHexString([IO.File]::ReadAllBytes($streamPath))) `
      -Expected '00010203FEFF' `
      -Message 'bounded stream bytes'
    Assert-NoComObject -Value $stream
    Remove-Item -LiteralPath $streamPath -Force -ErrorAction Stop

    $standardStreamPath = Join-Path $testRoot 'standard-stream.bin'
    $standardStream = Export-MsiBoundedStream `
      -SessionId $sessionId `
      -Table '_Streams' `
      -KeyColumn 'Name' `
      -StreamColumn 'Data' `
      -KeyValue 'fixture-standard-stream' `
      -OutputPath $standardStreamPath `
      -MaxStreamBytes 16
    Assert-Equal -Actual $standardStream.SizeBytes -Expected 6 -Message 'nullable standard _Streams schema is accepted for a non-null value'
    Assert-Equal `
      -Actual ([Convert]::ToHexString([IO.File]::ReadAllBytes($standardStreamPath))) `
      -Expected '00010203FEFF' `
      -Message 'standard _Streams bytes'
    Assert-NoComObject -Value $standardStream
    Remove-Item -LiteralPath $standardStreamPath -Force -ErrorAction Stop
  }
  Assert-DatabaseMoveRoundTrip -DatabasePath $msiPath -Suffix 'after-success'

  Invoke-WithQuerySession -DatabasePath $msiPath -Action {
    param($sessionId)

    [void](Invoke-ExpectedFailure -MessageContains 'at most one row' -Action {
      Get-MsiOptionalRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @((New-StringFilter -Column 'GroupKey' -Value 'duplicate')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'exactly one row' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'absent')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'unknown or duplicate' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'NotAColumn')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'single')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'type mismatch' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-IntColumn -Name 'TextValue')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'single')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'received i4' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryTypeMismatch' `
        -Columns @((New-StringColumn -Name 'ExpectedString')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'mismatch')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'nullability mismatch' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'NullableValue')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'single')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'filter type mismatch' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @([PSCustomObject]@{ Column = 'Id'; Kind = 'Int32'; Value = 1 }) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'filter value must be a string' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @([PSCustomObject]@{ Column = 'Id'; Kind = 'String'; Value = 1 }) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'must not be empty' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @((New-StringFilter -Column 'Id' -Value '')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'database-null sentinel' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @([PSCustomObject]@{
          Column = 'NumberValue'
          Kind = 'Int32'
          Value = [int]::MinValue
        }) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
    })
    [void](Invoke-ExpectedFailure -MessageContains 'not in the code-owned schema' -Action {
      Get-MsiSummaryString `
        -SessionId $sessionId `
        -PropertyId 8 `
        -MaxSize 128
    })
    [void](Invoke-ExpectedFailure -MessageContains 'summary property 7 exceeds' -Action {
      Get-MsiSummaryString `
        -SessionId $sessionId `
        -PropertyId 7 `
        -MaxSize 4
    })
    [void](Invoke-ExpectedFailure -MessageContains 'row cap' -Action {
      Invoke-MsiQuery `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -MaxRows 1 `
        -MaxAggregateCells 8 `
        -MaxAggregateUnits 4096
    })
    [void](Invoke-ExpectedFailure -MessageContains 'cell aggregate cap' -Action {
      Invoke-MsiQuery `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @(
          (New-StringColumn -Name 'Id'),
          (New-StringColumn -Name 'TextValue')
        ) `
        -MaxRows 1 `
        -MaxAggregateCells 1 `
        -MaxAggregateUnits 4096
    })
    [void](Invoke-ExpectedFailure -MessageContains 'field TextValue' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'TextValue' -MaxSize 8)) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'long')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 4096
    })
    [void](Invoke-ExpectedFailure -MessageContains 'unit aggregate cap' -Action {
      Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'TextValue')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'long')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 10
    })

    $oversizedOutput = Join-Path $testRoot 'oversized-stream.bin'
    [void](Invoke-ExpectedFailure -MessageContains 'outside' -Action {
      Export-MsiBoundedStream `
        -SessionId $sessionId `
        -Table '_FyAgentQueryStream' `
        -KeyColumn 'Id' `
        -StreamColumn 'Payload' `
        -KeyValue 'large' `
        -OutputPath $oversizedOutput `
        -MaxStreamBytes 8
    })
    Assert-True -Condition (-not (Test-Path -LiteralPath $oversizedOutput)) -Message 'rejected stream leaves no output'

    [void](Invoke-ExpectedFailure -MessageContains 'code-owned stream schema' -Action {
      Export-MsiBoundedStream `
        -SessionId $sessionId `
        -Table '_FyAgentQueryStream' `
        -KeyColumn 'Payload' `
        -StreamColumn 'Id' `
        -KeyValue 'small' `
        -OutputPath (Join-Path $testRoot 'invalid-stream-shape.bin') `
        -MaxStreamBytes 16
    })
  }
  Assert-DatabaseMoveRoundTrip -DatabasePath $msiPath -Suffix 'after-failure'

  for ($cycle = 0; $cycle -lt 8; $cycle += 1) {
    Invoke-WithQuerySession -DatabasePath $msiPath -Action {
      param($sessionId)
      $row = Get-MsiRequiredRow `
        -SessionId $sessionId `
        -Table '_FyAgentQueryFixture' `
        -Columns @((New-StringColumn -Name 'Id')) `
        -Filters @((New-StringFilter -Column 'Id' -Value 'single')) `
        -MaxAggregateCells 4 `
        -MaxAggregateUnits 1024
      Assert-Equal -Actual $row.Values[0] -Expected 'single' -Message 'repeated session query'
    }
  }
  Assert-DatabaseMoveRoundTrip -DatabasePath $msiPath -Suffix 'after-repeat'

  $closedSession = Open-MsiQuerySession -Path $msiPath
  Close-MsiQuerySession -SessionId $closedSession
  [void](Invoke-ExpectedFailure -MessageContains 'unknown or already closed' -Action {
    Close-MsiQuerySession -SessionId $closedSession
  })

  Remove-Item -LiteralPath $msiPath -Force -ErrorAction Stop
  Assert-True -Condition (-not (Test-Path -LiteralPath $msiPath)) -Message 'temporary MSI deletes immediately after all sessions close'
  $successMessage = "Windows Installer query integration OK: architecture=$actualArchitecture"
} catch {
  $testPrimaryError = $_.Exception
}

$testCleanupError = $null
if (Test-Path -LiteralPath $testRoot) {
  try {
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction Stop
  } catch {
    $testCleanupError = $_.Exception
  }
}
if ($null -ne $testPrimaryError) {
  if ($null -ne $testCleanupError) {
    throw [InvalidOperationException]::new(
      "$($testPrimaryError.Message) | test cleanup failed: $($testCleanupError.Message)",
      $testPrimaryError
    )
  }
  throw $testPrimaryError
}
if ($null -ne $testCleanupError) {
  throw $testCleanupError
}
Write-Output $successMessage
