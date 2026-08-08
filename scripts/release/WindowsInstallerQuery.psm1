Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$script:MsiQuerySessions = [Collections.Generic.Dictionary[string, object]]::new(
  [StringComparer]::Ordinal
)

# SQL identifiers are intentionally closed here. Callers may select a subset of
# these columns, but they cannot introduce a table or column name discovered at
# runtime from the candidate package.
$script:MsiQueryColumns = @{
  AppSearch = @{ Property = 'String'; Signature_ = 'String' }
  Binary = @{ Name = 'String' }
  Component = @{
    Component = 'String'
    Directory_ = 'String'
    Attributes = 'Int32'
    KeyPath = 'String'
  }
  ControlEvent = @{
    Dialog_ = 'String'
    Control_ = 'String'
    Event = 'String'
    Argument = 'String'
    Condition = 'String'
    Ordering = 'Int32'
  }
  CustomAction = @{
    Action = 'String'
    Type = 'Int32'
    Source = 'String'
    Target = 'String'
  }
  Dialog = @{ Dialog = 'String' }
  Directory = @{ Directory = 'String'; Directory_Parent = 'String' }
  Feature = @{ Feature = 'String' }
  FeatureComponents = @{ Feature_ = 'String'; Component_ = 'String' }
  File = @{
    File = 'String'
    Component_ = 'String'
    FileName = 'String'
    FileSize = 'Int32'
    Sequence = 'Int32'
  }
  InstallExecuteSequence = @{
    Action = 'String'
    Condition = 'String'
    Sequence = 'Int32'
  }
  InstallUISequence = @{
    Action = 'String'
    Condition = 'String'
    Sequence = 'Int32'
  }
  Media = @{ DiskId = 'Int32'; LastSequence = 'Int32'; Cabinet = 'String' }
  MsiLockPermissionsEx = @{ LockObject = 'String'; SDDLText = 'String' }
  Property = @{ Property = 'String'; Value = 'String' }
  Registry = @{ Registry = 'String'; Name = 'String' }
  RegLocator = @{
    Signature_ = 'String'
    Root = 'Int32'
    Key = 'String'
    Name = 'String'
    Type = 'Int32'
  }
  RemoveFile = @{
    FileKey = 'String'
    Component_ = 'String'
    FileName = 'String'
    DirProperty = 'String'
    InstallMode = 'Int32'
  }
  Shortcut = @{
    Shortcut = 'String'
    Directory_ = 'String'
    Component_ = 'String'
    Target = 'String'
  }
  _Streams = @{ Name = 'String'; Data = 'Stream' }

  # These schemas exist only so the native CI fixture exercises this exact
  # production adapter without committing an opaque binary MSI. The mismatch
  # schema deliberately expects String while its IDT declares i4 so ColumnInfo
  # rejection is covered on the real Automation boundary.
  _FyAgentQueryFixture = @{
    Id = 'String'
    GroupKey = 'String'
    TextValue = 'String'
    NumberValue = 'Int32'
    SmallNumber = 'Int32'
    NullableValue = 'String'
    EmptyValue = 'String'
  }
  _FyAgentQueryStream = @{ Id = 'String'; Payload = 'Stream' }
  _FyAgentQueryTypeMismatch = @{ Id = 'String'; ExpectedString = 'String' }
}

$script:MsiQueryTables = [Collections.Generic.HashSet[string]]::new(
  [StringComparer]::Ordinal
)
foreach ($tableName in $script:MsiQueryColumns.Keys) {
  [void]$script:MsiQueryTables.Add($tableName)
}

$script:MsiStreamQueries = @{
  _Streams = @{ KeyColumn = 'Name'; StreamColumn = 'Data'; Nullable = $true }
  _FyAgentQueryStream = @{ KeyColumn = 'Id'; StreamColumn = 'Payload'; Nullable = $false }
}

$script:MaximumColumnCount = 16
$script:MaximumRowCount = 32768
$script:MaximumFieldUnits = 1MB
$script:MaximumAggregateCells = 524288
$script:MaximumAggregateUnits = 256MB
$script:MaximumStreamBytes = 1GB

function Add-MsiCleanupError {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [Collections.Generic.List[string]]$Errors,

    [Parameter(Mandatory = $true)]
    [string]$Description,

    [Parameter(Mandatory = $true)]
    [Exception]$Exception
  )

  [void]$Errors.Add("${Description}: $($Exception.Message)")
}

function Release-MsiComObject {
  param(
    [AllowNull()]
    [object]$Value,

    [Parameter(Mandatory = $true)]
    [string]$Description,

    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [Collections.Generic.List[string]]$Errors
  )

  if ($null -eq $Value) {
    return
  }
  if (-not [Runtime.InteropServices.Marshal]::IsComObject($Value)) {
    [void]$Errors.Add("${Description}: owned value is not a COM object")
    return
  }
  try {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Value)
  } catch {
    Add-MsiCleanupError -Errors $Errors -Description $Description -Exception $_.Exception
  }
}

function Throw-MsiOperationFailure {
  param(
    [AllowNull()]
    [Exception]$PrimaryError,

    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [Collections.Generic.List[string]]$CleanupErrors,

    [Parameter(Mandatory = $true)]
    [string]$Context
  )

  $cleanupMessage = $CleanupErrors -join '; '
  if ($null -ne $PrimaryError) {
    if ($CleanupErrors.Count -gt 0) {
      throw [InvalidOperationException]::new(
        "$($PrimaryError.Message) | $Context cleanup failed: $cleanupMessage",
        $PrimaryError
      )
    }
    throw $PrimaryError
  }
  if ($CleanupErrors.Count -gt 0) {
    throw [InvalidOperationException]::new("$Context cleanup failed: $cleanupMessage")
  }
}

function Assert-MsiExactProperties {
  param(
    [Parameter(Mandatory = $true)]
    [object]$Value,

    [Parameter(Mandatory = $true)]
    [string[]]$Expected,

    [Parameter(Mandatory = $true)]
    [string]$Description
  )

  if ($Value -isnot [PSCustomObject]) {
    throw "$Description must be a PSCustomObject"
  }
  $actual = @($Value.PSObject.Properties.Name | Sort-Object)
  $expectedSorted = @($Expected | Sort-Object)
  if (
    $actual.Count -ne $expectedSorted.Count -or
    [string]::Join("`n", $actual) -cne [string]::Join("`n", $expectedSorted)
  ) {
    throw "$Description must contain exactly: $($Expected -join ', ')"
  }
}

function Get-MsiAllowedColumnSet {
  param([Parameter(Mandatory = $true)][string]$Table)

  if (-not $script:MsiQueryTables.Contains($Table)) {
    throw "MSI query table is not in the code-owned allowlist: $Table"
  }
  $allowed = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  foreach ($name in $script:MsiQueryColumns[$Table].Keys) {
    [void]$allowed.Add($name)
  }
  Write-Output -NoEnumerate $allowed
}

function Get-MsiColumnKind {
  param(
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][string]$Column
  )

  $allowed = Get-MsiAllowedColumnSet -Table $Table
  if (-not $allowed.Contains($Column)) {
    throw "MSI query column is not in the code-owned schema for ${Table}: $Column"
  }
  return [string]$script:MsiQueryColumns[$Table][$Column]
}

function Assert-MsiQueryBudgets {
  param(
    [Parameter(Mandatory = $true)][int]$MaxRows,
    [Parameter(Mandatory = $true)][int]$MaxAggregateCells,
    [Parameter(Mandatory = $true)][long]$MaxAggregateUnits
  )

  if ($MaxRows -lt 1 -or $MaxRows -gt $script:MaximumRowCount) {
    throw "MSI query row cap must be between 1 and $($script:MaximumRowCount): $MaxRows"
  }
  if (
    $MaxAggregateCells -lt 1 -or
    $MaxAggregateCells -gt $script:MaximumAggregateCells
  ) {
    throw "MSI query aggregate-cell cap must be between 1 and $($script:MaximumAggregateCells): $MaxAggregateCells"
  }
  if (
    $MaxAggregateUnits -lt 1 -or
    $MaxAggregateUnits -gt $script:MaximumAggregateUnits
  ) {
    throw "MSI query aggregate-unit cap must be between 1 and $($script:MaximumAggregateUnits): $MaxAggregateUnits"
  }
}

function Assert-MsiColumns {
  param(
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][object[]]$Columns
  )

  if ($Columns.Count -lt 1 -or $Columns.Count -gt $script:MaximumColumnCount) {
    throw "MSI query column count must be between 1 and $($script:MaximumColumnCount): $($Columns.Count)"
  }
  $allowed = Get-MsiAllowedColumnSet -Table $Table
  $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  foreach ($column in $Columns) {
    Assert-MsiExactProperties -Value $column -Expected @(
      'Name',
      'Kind',
      'Nullable',
      'MaxSize'
    ) -Description 'MSI query column descriptor'
    if (
      $column.Name -isnot [string] -or
      -not $allowed.Contains($column.Name) -or
      -not $seen.Add($column.Name)
    ) {
      throw "MSI query contains an unknown or duplicate code-owned column for ${Table}: $($column.Name)"
    }
    if ($column.Kind -cnotin @('String', 'Int32')) {
      throw "MSI query column kind must be String or Int32: $($column.Name)"
    }
    $expectedKind = Get-MsiColumnKind -Table $Table -Column $column.Name
    if ($column.Kind -cne $expectedKind) {
      throw "MSI query column $($column.Name) type mismatch: expected $expectedKind, received $($column.Kind)"
    }
    if ($column.Nullable -isnot [bool]) {
      throw "MSI query column Nullable must be Boolean: $($column.Name)"
    }
    if ($column.MaxSize -isnot [int]) {
      throw "MSI query column MaxSize must be Int32: $($column.Name)"
    }
    if ($column.Kind -ceq 'String') {
      if ($column.MaxSize -lt 1 -or $column.MaxSize -gt $script:MaximumFieldUnits) {
        throw "MSI string field cap is invalid for $($column.Name): $($column.MaxSize)"
      }
    } elseif ($column.MaxSize -ne 4) {
      throw "MSI Int32 field cap must be exactly 4 for $($column.Name)"
    }
  }
}

function Assert-MsiFilters {
  param(
    [Parameter(Mandatory = $true)][string]$Table,
    [AllowEmptyCollection()][object[]]$Filters
  )

  $allowed = Get-MsiAllowedColumnSet -Table $Table
  $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  foreach ($filter in $Filters) {
    Assert-MsiExactProperties -Value $filter -Expected @(
      'Column',
      'Kind',
      'Value'
    ) -Description 'MSI query filter descriptor'
    if (
      $filter.Column -isnot [string] -or
      -not $allowed.Contains($filter.Column) -or
      -not $seen.Add($filter.Column)
    ) {
      throw "MSI query contains an unknown or duplicate filter column for ${Table}: $($filter.Column)"
    }
    if ($filter.Kind -ceq 'String') {
      $expectedKind = Get-MsiColumnKind -Table $Table -Column $filter.Column
      if ($expectedKind -cne 'String') {
        throw "MSI query filter type mismatch for $($filter.Column): expected $expectedKind, received String"
      }
      if ($filter.Value -isnot [string]) {
        throw "MSI String filter value must be a string: $($filter.Column)"
      }
      if ($filter.Value.Length -eq 0) {
        throw "MSI String filter value must not be empty because Automation binds it as null: $($filter.Column)"
      }
      if ($filter.Value.Length -gt $script:MaximumFieldUnits) {
        throw "MSI String filter value is too large: $($filter.Column)"
      }
    } elseif ($filter.Kind -ceq 'Int32') {
      $expectedKind = Get-MsiColumnKind -Table $Table -Column $filter.Column
      if ($expectedKind -cne 'Int32') {
        throw "MSI query filter type mismatch for $($filter.Column): expected $expectedKind, received Int32"
      }
      if ($filter.Value -isnot [int]) {
        throw "MSI Int32 filter value must be Int32: $($filter.Column)"
      }
      if ($filter.Value -eq [int]::MinValue) {
        throw "MSI Int32 filter value must not use the database-null sentinel: $($filter.Column)"
      }
    } else {
      throw "MSI query filter kind must be String or Int32: $($filter.Column)"
    }
  }
}

function Get-MsiQuerySession {
  param([Parameter(Mandatory = $true)][string]$SessionId)

  $session = $null
  if (-not $script:MsiQuerySessions.TryGetValue($SessionId, [ref]$session)) {
    throw "MSI query session is unknown or already closed: $SessionId"
  }
  return $session
}

function New-MsiSelectCommand {
  param(
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][object[]]$Columns,
    [AllowEmptyCollection()][object[]]$Filters
  )

  $quotedColumns = @($Columns | ForEach-Object { "``$($_.Name)``" })
  $command = "SELECT $($quotedColumns -join ', ') FROM ``$Table``"
  if ($Filters.Count -gt 0) {
    $predicates = @($Filters | ForEach-Object { "``$($_.Column)`` = ?" })
    $command += " WHERE $($predicates -join ' AND ')"
  }
  return $command
}

function New-MsiParameterRecord {
  param(
    [Parameter(Mandatory = $true)][object]$Installer,
    [AllowEmptyCollection()][object[]]$Filters
  )

  if ($Filters.Count -eq 0) {
    return $null
  }
  $record = $Installer.CreateRecord($Filters.Count)
  try {
    for ($index = 1; $index -le $Filters.Count; $index += 1) {
      $filter = $Filters[$index - 1]
      if ($filter.Kind -ceq 'String') {
        $record.StringData($index) = [string]$filter.Value
      } else {
        $record.IntegerData($index) = [int]$filter.Value
      }
    }
    return $record
  } catch {
    $primaryError = $_.Exception
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    Release-MsiComObject -Value $record -Description 'parameter Record' -Errors $cleanupErrors
    Throw-MsiOperationFailure -PrimaryError $primaryError -CleanupErrors $cleanupErrors -Context 'MSI parameter construction'
  }
}

function Assert-MsiColumnType {
  param(
    [Parameter(Mandatory = $true)][object]$Column,
    [Parameter(Mandatory = $true)][string]$ActualType
  )

  $matchesKind = if ($Column.Kind -ceq 'String') {
    $ActualType -cmatch '^[sSlL][0-9]+$'
  } else {
    $ActualType -cmatch '^[iI][24]$'
  }
  if (-not $matchesKind) {
    throw "MSI column $($Column.Name) type mismatch: expected $($Column.Kind), received $ActualType"
  }
  $actualNullable = [char]::IsUpper($ActualType[0])
  if ($actualNullable -ne $Column.Nullable) {
    throw "MSI column $($Column.Name) nullability mismatch: expected $($Column.Nullable), received $actualNullable ($ActualType)"
  }
}

function Assert-MsiProjectionMetadata {
  param(
    [Parameter(Mandatory = $true)][object]$NameRecord,
    [Parameter(Mandatory = $true)][object]$TypeRecord,
    [Parameter(Mandatory = $true)][object[]]$Columns
  )

  for ($index = 1; $index -le $Columns.Count; $index += 1) {
    $column = $Columns[$index - 1]
    if ([bool]$NameRecord.IsNull($index)) {
      throw "MSI projection metadata has a null name at ordinal $index"
    }
    [int]$nameSize = $NameRecord.DataSize($index)
    if ($nameSize -lt 1 -or $nameSize -gt 128) {
      throw "MSI projection metadata name size is invalid at ordinal ${index}: $nameSize"
    }
    [string]$actualName = $NameRecord.StringData($index)
    if ($actualName.Length -ne $nameSize) {
      throw "MSI projection metadata name at ordinal $index materialized $($actualName.Length) units after declaring $nameSize"
    }
    if ($actualName -cne $column.Name) {
      throw "MSI projection name mismatch at ordinal ${index}: expected $($column.Name), received $actualName"
    }

    if ([bool]$TypeRecord.IsNull($index)) {
      throw "MSI projection metadata has a null type at ordinal $index"
    }
    [int]$typeSize = $TypeRecord.DataSize($index)
    if ($typeSize -lt 2 -or $typeSize -gt 16) {
      throw "MSI projection metadata type size is invalid at ordinal ${index}: $typeSize"
    }
    [string]$actualType = $TypeRecord.StringData($index)
    if ($actualType.Length -ne $typeSize) {
      throw "MSI projection metadata type at ordinal $index materialized $($actualType.Length) units after declaring $typeSize"
    }
    Assert-MsiColumnType -Column $column -ActualType $actualType
  }
}

function Copy-MsiRecordValues {
  param(
    [Parameter(Mandatory = $true)][object]$Record,
    [Parameter(Mandatory = $true)][object[]]$Columns,
    [Parameter(Mandatory = $true)][ref]$AggregateCells,
    [Parameter(Mandatory = $true)][ref]$AggregateUnits,
    [Parameter(Mandatory = $true)][int]$MaxAggregateCells,
    [Parameter(Mandatory = $true)][long]$MaxAggregateUnits
  )

  $values = [object[]]::new($Columns.Count)
  for ($index = 1; $index -le $Columns.Count; $index += 1) {
    $column = $Columns[$index - 1]
    if ($AggregateCells.Value -ge $MaxAggregateCells) {
      throw "MSI query exceeds cell aggregate cap $MaxAggregateCells"
    }
    $AggregateCells.Value = [int]$AggregateCells.Value + 1

    [bool]$isNull = $Record.IsNull($index)
    if ($isNull) {
      if (-not $column.Nullable) {
        throw "MSI query returned null for non-nullable column $($column.Name)"
      }
      $values[$index - 1] = $null
      continue
    }

    [long]$dataSize = $Record.DataSize($index)
    if ($dataSize -lt 0 -or $dataSize -gt $column.MaxSize) {
      throw "MSI query field $($column.Name) exceeds its $($column.MaxSize)-unit cap: $dataSize"
    }
    if ($AggregateUnits.Value -gt $MaxAggregateUnits - $dataSize) {
      throw "MSI query exceeds unit aggregate cap $MaxAggregateUnits"
    }
    $AggregateUnits.Value = [long]$AggregateUnits.Value + $dataSize

    if ($column.Kind -ceq 'String') {
      [string]$stringValue = [string]$Record.StringData($index)
      if ($stringValue.Length -ne $dataSize) {
        throw "MSI query string column $($column.Name) materialized $($stringValue.Length) units after declaring $dataSize"
      }
      $values[$index - 1] = $stringValue
    } else {
      if ($dataSize -ne 4) {
        throw "MSI query integer column $($column.Name) has unexpected DataSize $dataSize"
      }
      [int]$integerValue = [int]$Record.IntegerData($index)
      if ($integerValue -eq [int]::MinValue) {
        throw "MSI query integer column $($column.Name) returned the database-null sentinel"
      }
      $values[$index - 1] = $integerValue
    }
  }
  return [PSCustomObject]@{ Values = $values }
}

function Invoke-MsiQueryCore {
  param(
    [Parameter(Mandatory = $true)][string]$SessionId,
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][object[]]$Columns,
    [AllowEmptyCollection()][object[]]$Filters = @(),
    [Parameter(Mandatory = $true)][int]$MaxRows,
    [Parameter(Mandatory = $true)][int]$MaxAggregateCells,
    [Parameter(Mandatory = $true)][long]$MaxAggregateUnits
  )

  Assert-MsiQueryBudgets -MaxRows $MaxRows -MaxAggregateCells $MaxAggregateCells -MaxAggregateUnits $MaxAggregateUnits
  Assert-MsiColumns -Table $Table -Columns $Columns
  Assert-MsiFilters -Table $Table -Filters $Filters

  $session = Get-MsiQuerySession -SessionId $SessionId
  $command = New-MsiSelectCommand -Table $Table -Columns $Columns -Filters $Filters
  $rows = [Collections.Generic.List[object]]::new()
  [int]$aggregateCells = 0
  [long]$aggregateUnits = 0
  $view = $null
  $parameterRecord = $null
  $nameRecord = $null
  $typeRecord = $null
  $record = $null
  $primaryError = $null

  try {
    $view = $session.Database.OpenView($command)
    $parameterRecord = New-MsiParameterRecord -Installer $session.Installer -Filters $Filters
    if ($null -eq $parameterRecord) {
      [void]$view.Execute()
    } else {
      [void]$view.Execute($parameterRecord)
    }
    $nameRecord = $view.ColumnInfo(0)
    $typeRecord = $view.ColumnInfo(1)
    Assert-MsiProjectionMetadata -NameRecord $nameRecord -TypeRecord $typeRecord -Columns $Columns

    while ($true) {
      $record = $view.Fetch()
      if ($null -eq $record) {
        break
      }
      $rowPrimaryError = $null
      $row = $null
      try {
        if ($rows.Count -ge $MaxRows) {
          throw "MSI query exceeds row cap $MaxRows for table $Table"
        }
        $row = Copy-MsiRecordValues `
          -Record $record `
          -Columns $Columns `
          -AggregateCells ([ref]$aggregateCells) `
          -AggregateUnits ([ref]$aggregateUnits) `
          -MaxAggregateCells $MaxAggregateCells `
          -MaxAggregateUnits $MaxAggregateUnits
      } catch {
        $rowPrimaryError = $_.Exception
      }
      $rowCleanupErrors = [Collections.Generic.List[string]]::new()
      Release-MsiComObject -Value $record -Description 'fetched Record' -Errors $rowCleanupErrors
      $record = $null
      Throw-MsiOperationFailure -PrimaryError $rowPrimaryError -CleanupErrors $rowCleanupErrors -Context 'MSI row copy'
      [void]$rows.Add($row)
    }
  } catch {
    $primaryError = $_.Exception
  }

  $cleanupErrors = [Collections.Generic.List[string]]::new()
  Release-MsiComObject -Value $record -Description 'fetched Record' -Errors $cleanupErrors
  $record = $null
  Release-MsiComObject -Value $typeRecord -Description 'column-type Record' -Errors $cleanupErrors
  $typeRecord = $null
  Release-MsiComObject -Value $nameRecord -Description 'column-name Record' -Errors $cleanupErrors
  $nameRecord = $null
  Release-MsiComObject -Value $parameterRecord -Description 'parameter Record' -Errors $cleanupErrors
  $parameterRecord = $null
  if ($null -ne $view) {
    try {
      [void]$view.Close()
    } catch {
      Add-MsiCleanupError -Errors $cleanupErrors -Description 'View.Close' -Exception $_.Exception
    }
  }
  Release-MsiComObject -Value $view -Description 'View' -Errors $cleanupErrors
  $view = $null
  Throw-MsiOperationFailure -PrimaryError $primaryError -CleanupErrors $cleanupErrors -Context "MSI query $Table"

  return $rows.ToArray()
}

function Open-MsiQuerySession {
  [CmdletBinding()]
  param([Parameter(Mandatory = $true)][string]$Path)

  $resolvedPath = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
  if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
    throw "MSI query path is not a regular file: $resolvedPath"
  }

  $installer = $null
  $database = $null
  $primaryError = $null
  $sessionId = [Guid]::NewGuid().ToString('N')
  try {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase($resolvedPath, 0)
    $script:MsiQuerySessions.Add(
      $sessionId,
      [PSCustomObject]@{
        Installer = $installer
        Database = $database
        Path = [string]$resolvedPath
      }
    )
    return $sessionId
  } catch {
    $primaryError = $_.Exception
  }

  $cleanupErrors = [Collections.Generic.List[string]]::new()
  Release-MsiComObject -Value $database -Description 'Database' -Errors $cleanupErrors
  Release-MsiComObject -Value $installer -Description 'Installer' -Errors $cleanupErrors
  Throw-MsiOperationFailure -PrimaryError $primaryError -CleanupErrors $cleanupErrors -Context 'MSI session open'
}

function Close-MsiQuerySession {
  [CmdletBinding()]
  param([Parameter(Mandatory = $true)][string]$SessionId)

  $session = Get-MsiQuerySession -SessionId $SessionId
  [void]$script:MsiQuerySessions.Remove($SessionId)
  $database = $session.Database
  $installer = $session.Installer
  $session.Database = $null
  $session.Installer = $null

  $cleanupErrors = [Collections.Generic.List[string]]::new()
  Release-MsiComObject -Value $database -Description 'Database' -Errors $cleanupErrors
  Release-MsiComObject -Value $installer -Description 'Installer' -Errors $cleanupErrors
  Throw-MsiOperationFailure -PrimaryError $null -CleanupErrors $cleanupErrors -Context 'MSI session close'
}

function Invoke-MsiQuery {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)][string]$SessionId,
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][object[]]$Columns,
    [AllowEmptyCollection()][object[]]$Filters = @(),
    [Parameter(Mandatory = $true)][int]$MaxRows,
    [Parameter(Mandatory = $true)][int]$MaxAggregateCells,
    [Parameter(Mandatory = $true)][long]$MaxAggregateUnits
  )

  return Invoke-MsiQueryCore @PSBoundParameters
}

function Get-MsiOptionalRow {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)][string]$SessionId,
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][object[]]$Columns,
    [AllowEmptyCollection()][object[]]$Filters = @(),
    [Parameter(Mandatory = $true)][int]$MaxAggregateCells,
    [Parameter(Mandatory = $true)][long]$MaxAggregateUnits
  )

  $rows = @(
    Invoke-MsiQueryCore `
      -SessionId $SessionId `
      -Table $Table `
      -Columns $Columns `
      -Filters $Filters `
      -MaxRows 2 `
      -MaxAggregateCells $MaxAggregateCells `
      -MaxAggregateUnits $MaxAggregateUnits
  )
  if ($rows.Count -gt 1) {
    throw "MSI query expected at most one row in $Table; found $($rows.Count)"
  }
  if ($rows.Count -eq 0) {
    return $null
  }
  return $rows[0]
}

function Get-MsiRequiredRow {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)][string]$SessionId,
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][object[]]$Columns,
    [AllowEmptyCollection()][object[]]$Filters = @(),
    [Parameter(Mandatory = $true)][int]$MaxAggregateCells,
    [Parameter(Mandatory = $true)][long]$MaxAggregateUnits
  )

  $row = Get-MsiOptionalRow @PSBoundParameters
  if ($null -eq $row) {
    throw "MSI query expected exactly one row in $Table; found 0"
  }
  return $row
}

function Assert-MsiStreamMetadata {
  param(
    [Parameter(Mandatory = $true)][object]$NameRecord,
    [Parameter(Mandatory = $true)][object]$TypeRecord,
    [Parameter(Mandatory = $true)][string]$StreamColumn,
    [Parameter(Mandatory = $true)][bool]$Nullable
  )

  if ([bool]$NameRecord.IsNull(1) -or [bool]$TypeRecord.IsNull(1)) {
    throw 'MSI stream projection metadata is null'
  }
  [int]$nameSize = $NameRecord.DataSize(1)
  [int]$typeSize = $TypeRecord.DataSize(1)
  if ($nameSize -lt 1 -or $nameSize -gt 128 -or $typeSize -lt 2 -or $typeSize -gt 16) {
    throw 'MSI stream projection metadata size is invalid'
  }
  [string]$actualName = $NameRecord.StringData(1)
  [string]$actualType = $TypeRecord.StringData(1)
  if ($actualName.Length -ne $nameSize -or $actualType.Length -ne $typeSize) {
    throw 'MSI stream projection metadata materialized a value with an unexpected size'
  }
  $actualNullable = $actualType -cmatch '^V0$'
  if (
    $actualName -cne $StreamColumn -or
    $actualType -cnotmatch '^[vV]0$' -or
    $actualNullable -ne $Nullable
  ) {
    $expectedType = if ($Nullable) { 'V0' } else { 'v0' }
    throw "MSI stream projection mismatch: expected $StreamColumn/$expectedType, received $actualName/$actualType"
  }
}

function Export-MsiBoundedStream {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)][string]$SessionId,
    [Parameter(Mandatory = $true)][string]$Table,
    [Parameter(Mandatory = $true)][string]$KeyColumn,
    [Parameter(Mandatory = $true)][string]$StreamColumn,
    [Parameter(Mandatory = $true)][string]$KeyValue,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][int]$MaxStreamBytes
  )

  if ($MaxStreamBytes -lt 1 -or $MaxStreamBytes -gt $script:MaximumStreamBytes) {
    throw "MSI stream byte cap must be between 1 and $($script:MaximumStreamBytes): $MaxStreamBytes"
  }
  if ($KeyValue.Length -eq 0) {
    throw 'MSI stream key must not be empty because Automation binds it as null'
  }
  if ($KeyValue.Length -gt $script:MaximumFieldUnits) {
    throw 'MSI stream key is too large'
  }
  if (
    -not $script:MsiQueryTables.Contains($Table) -or
    -not $script:MsiStreamQueries.ContainsKey($Table)
  ) {
    throw "MSI stream table is not in the code-owned stream schema: $Table"
  }
  $streamQuery = $script:MsiStreamQueries[$Table]
  if (
    $KeyColumn -cne $streamQuery.KeyColumn -or
    $StreamColumn -cne $streamQuery.StreamColumn
  ) {
    throw "MSI stream identifiers do not match the code-owned stream schema for $Table"
  }
  if (
    (Get-MsiColumnKind -Table $Table -Column $KeyColumn) -cne 'String' -or
    (Get-MsiColumnKind -Table $Table -Column $StreamColumn) -cne 'Stream'
  ) {
    throw "MSI stream schema has an invalid key or payload kind for $Table"
  }
  $resolvedParent = (Resolve-Path -LiteralPath (Split-Path -Parent $OutputPath) -ErrorAction Stop).Path
  $resolvedOutput = [IO.Path]::GetFullPath($OutputPath)
  if (Test-Path -LiteralPath $resolvedOutput) {
    throw "MSI stream output already exists: $resolvedOutput"
  }
  if (-not [string]::Equals(
      [IO.Path]::GetDirectoryName($resolvedOutput),
      $resolvedParent,
      [StringComparison]::OrdinalIgnoreCase
    )) {
    throw "MSI stream output must be an exact child of its resolved parent: $resolvedOutput"
  }

  $session = Get-MsiQuerySession -SessionId $SessionId
  $command = "SELECT ``$StreamColumn`` FROM ``$Table`` WHERE ``$KeyColumn`` = ?"
  $filter = [PSCustomObject]@{ Column = $KeyColumn; Kind = 'String'; Value = $KeyValue }
  $view = $null
  $parameterRecord = $null
  $nameRecord = $null
  $typeRecord = $null
  $record = $null
  $duplicateRecord = $null
  $output = $null
  $createdOutput = $false
  $primaryError = $null
  [int]$streamSize = 0
  try {
    $view = $session.Database.OpenView($command)
    $parameterRecord = New-MsiParameterRecord -Installer $session.Installer -Filters @($filter)
    [void]$view.Execute($parameterRecord)
    $nameRecord = $view.ColumnInfo(0)
    $typeRecord = $view.ColumnInfo(1)
    Assert-MsiStreamMetadata `
      -NameRecord $nameRecord `
      -TypeRecord $typeRecord `
      -StreamColumn $StreamColumn `
      -Nullable ([bool]$streamQuery.Nullable)
    $record = $view.Fetch()
    if ($null -eq $record) {
      throw "MSI stream row is missing for $Table.$KeyColumn=$KeyValue"
    }
    $duplicateRecord = $view.Fetch()
    if ($null -ne $duplicateRecord) {
      throw "MSI stream query returned more than one row for $Table.$KeyColumn=$KeyValue"
    }
    if ([bool]$record.IsNull(1)) {
      throw "MSI stream $Table.$StreamColumn is null for key $KeyValue"
    }
    [long]$declaredSize = $record.DataSize(1)
    if ($declaredSize -lt 1 -or $declaredSize -gt $MaxStreamBytes) {
      throw "MSI stream size $declaredSize is outside the 1..$MaxStreamBytes byte policy"
    }
    $streamSize = [int]$declaredSize

    $output = [IO.File]::Open(
      $resolvedOutput,
      [IO.FileMode]::CreateNew,
      [IO.FileAccess]::Write,
      [IO.FileShare]::None
    )
    $createdOutput = $true
    $latin1 = [Text.Encoding]::GetEncoding(
      28591,
      [Text.EncoderFallback]::ExceptionFallback,
      [Text.DecoderFallback]::ExceptionFallback
    )
    [int]$remaining = $streamSize
    while ($remaining -gt 0) {
      [int]$requested = [Math]::Min(1MB, $remaining)
      [string]$chunk = $record.ReadStream(1, $requested, 1)
      if ([string]::IsNullOrEmpty($chunk)) {
        throw "MSI stream ended before its declared $streamSize bytes"
      }
      [byte[]]$bytes = $latin1.GetBytes($chunk)
      if ($bytes.Length -ne $chunk.Length -or $bytes.Length -gt $remaining) {
        throw 'MSI stream returned an invalid chunk size'
      }
      $output.Write($bytes, 0, $bytes.Length)
      $remaining -= $bytes.Length
    }
    [string]$extra = $record.ReadStream(1, 1, 1)
    if (-not [string]::IsNullOrEmpty($extra)) {
      throw 'MSI stream exceeds its declared size'
    }
  } catch {
    $primaryError = $_.Exception
  }

  $cleanupErrors = [Collections.Generic.List[string]]::new()
  if ($null -ne $output) {
    try {
      $output.Dispose()
    } catch {
      Add-MsiCleanupError -Errors $cleanupErrors -Description 'stream output Dispose' -Exception $_.Exception
    }
    $output = $null
  }
  Release-MsiComObject -Value $duplicateRecord -Description 'duplicate stream Record' -Errors $cleanupErrors
  $duplicateRecord = $null
  Release-MsiComObject -Value $record -Description 'stream Record' -Errors $cleanupErrors
  $record = $null
  Release-MsiComObject -Value $typeRecord -Description 'stream column-type Record' -Errors $cleanupErrors
  $typeRecord = $null
  Release-MsiComObject -Value $nameRecord -Description 'stream column-name Record' -Errors $cleanupErrors
  $nameRecord = $null
  Release-MsiComObject -Value $parameterRecord -Description 'stream parameter Record' -Errors $cleanupErrors
  $parameterRecord = $null
  if ($null -ne $view) {
    try {
      [void]$view.Close()
    } catch {
      Add-MsiCleanupError -Errors $cleanupErrors -Description 'stream View.Close' -Exception $_.Exception
    }
  }
  Release-MsiComObject -Value $view -Description 'stream View' -Errors $cleanupErrors
  $view = $null

  if ($null -ne $primaryError -or $cleanupErrors.Count -gt 0) {
    if ($createdOutput -and (Test-Path -LiteralPath $resolvedOutput)) {
      try {
        Remove-Item -LiteralPath $resolvedOutput -Force -ErrorAction Stop
      } catch {
        Add-MsiCleanupError -Errors $cleanupErrors -Description 'partial stream output removal' -Exception $_.Exception
      }
    }
    Throw-MsiOperationFailure -PrimaryError $primaryError -CleanupErrors $cleanupErrors -Context "MSI stream export $Table"
  }

  return [PSCustomObject]@{
    Path = [string]$resolvedOutput
    SizeBytes = [int]$streamSize
  }
}

function Get-MsiSummaryString {
  [CmdletBinding()]
  param(
    [Parameter(Mandatory = $true)][string]$SessionId,
    [Parameter(Mandatory = $true)][int]$PropertyId,
    [Parameter(Mandatory = $true)][int]$MaxSize
  )

  if ($PropertyId -ne 7) {
    throw "MSI summary property is not in the code-owned schema: $PropertyId"
  }
  if ($MaxSize -lt 1 -or $MaxSize -gt $script:MaximumFieldUnits) {
    throw "MSI summary string cap is invalid: $MaxSize"
  }
  $session = Get-MsiQuerySession -SessionId $SessionId
  $summary = $null
  $primaryError = $null
  $value = $null
  try {
    $summary = $session.Database.SummaryInformation(0)
    $value = [string]$summary.Property($PropertyId)
    if ($value.Length -gt $MaxSize) {
      throw "MSI summary property $PropertyId exceeds its $MaxSize-unit cap"
    }
  } catch {
    $primaryError = $_.Exception
  }
  $cleanupErrors = [Collections.Generic.List[string]]::new()
  Release-MsiComObject -Value $summary -Description 'SummaryInformation' -Errors $cleanupErrors
  $summary = $null
  Throw-MsiOperationFailure -PrimaryError $primaryError -CleanupErrors $cleanupErrors -Context 'MSI summary query'
  return [string]$value
}

Export-ModuleMember -Function @(
  'Open-MsiQuerySession',
  'Close-MsiQuerySession',
  'Invoke-MsiQuery',
  'Get-MsiOptionalRow',
  'Get-MsiRequiredRow',
  'Export-MsiBoundedStream',
  'Get-MsiSummaryString'
)
