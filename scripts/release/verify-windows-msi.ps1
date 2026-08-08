[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$MsiPath,

  [Parameter(Mandatory = $true)]
  [string]$InstallerActionsDll,

  [Parameter(Mandatory = $true)]
  [ValidateSet('x64', 'arm64')]
  [string]$Architecture,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+$')]
  [string]$AppVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Release-ComObject {
  param([object]$Value)

  if ($null -ne $Value -and [Runtime.InteropServices.Marshal]::IsComObject($Value)) {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Value)
  }
}

function Get-PeMachine {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [string]$Description
  )

  [byte[]]$bytes = [IO.File]::ReadAllBytes($Path)
  if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
    throw "$Description is not a PE file: $Path"
  }

  [uint32]$peOffset = [BitConverter]::ToUInt32($bytes, 0x3c)
  if (
    $peOffset -lt 0x40 -or
    $peOffset -gt ($bytes.Length - 24) -or
    $bytes[$peOffset] -ne 0x50 -or
    $bytes[$peOffset + 1] -ne 0x45 -or
    $bytes[$peOffset + 2] -ne 0x00 -or
    $bytes[$peOffset + 3] -ne 0x00
  ) {
    throw "$Description has no valid PE header: $Path"
  }

  [uint16]$characteristics = [BitConverter]::ToUInt16($bytes, $peOffset + 22)
  if (($characteristics -band 0x2000) -eq 0) {
    throw "$Description PE header is not marked as a DLL: $Path"
  }

  return [BitConverter]::ToUInt16($bytes, $peOffset + 4)
}

$resolvedMsi = (Resolve-Path -LiteralPath $MsiPath -ErrorAction Stop).Path
$resolvedHelper = (Resolve-Path -LiteralPath $InstallerActionsDll -ErrorAction Stop).Path
if (-not (Test-Path -LiteralPath $resolvedMsi -PathType Leaf)) {
  throw "Windows MSI not found: $resolvedMsi"
}
if (-not (Test-Path -LiteralPath $resolvedHelper -PathType Leaf)) {
  throw "installer-actions DLL not found: $resolvedHelper"
}

[uint16]$expectedMachine = if ($Architecture -eq 'arm64') { 0xAA64 } else { 0x8664 }
$expectedMachineName = if ($Architecture -eq 'arm64') { 'ARM64' } else { 'x64' }
$helperMachine = Get-PeMachine -Path $resolvedHelper -Description 'Built installer-actions DLL'
if ($helperMachine -ne $expectedMachine) {
  throw "Built installer-actions DLL machine is 0x$($helperMachine.ToString('X4')); expected $expectedMachineName"
}

$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) "fyagent-msi-verify-$([Guid]::NewGuid().ToString('N'))"
$installer = $null
$database = $null
New-Item -ItemType Directory -Path $temporaryRoot -ErrorAction Stop | Out-Null

try {
  $installer = New-Object -ComObject WindowsInstaller.Installer
  # Open mode 0 is read-only. Verification must never mutate the candidate MSI.
  $database = $installer.OpenDatabase($resolvedMsi, 0)

  function Get-MsiRows {
    param(
      [Parameter(Mandatory = $true)]
      [string]$Query,

      [Parameter(Mandatory = $true)]
      [string[]]$Columns
    )

    $rows = [Collections.Generic.List[object]]::new()
    $view = $null
    $record = $null
    try {
      $view = $database.OpenView($Query)
      $view.Execute()
      while ($null -ne ($record = $view.Fetch())) {
        $values = [ordered]@{}
        for ($index = 0; $index -lt $Columns.Count; $index += 1) {
          $values[$Columns[$index]] = [string]$record.StringData($index + 1)
        }
        $rows.Add([PSCustomObject]$values)
        Release-ComObject $record
        $record = $null
      }
    } finally {
      Release-ComObject $record
      if ($null -ne $view) {
        try {
          $view.Close()
        } catch {
          Write-Warning "Failed to close a read-only MSI query view: $($_.Exception.Message)"
        }
      }
      Release-ComObject $view
    }
    return $rows.ToArray()
  }

  function Get-RequiredMsiProperty {
    param([Parameter(Mandatory = $true)][string]$Name)

    $rows = @(Get-MsiRows `
      -Query "SELECT ``Value`` FROM ``Property`` WHERE ``Property``='$Name'" `
      -Columns @('Value'))
    if ($rows.Count -ne 1) {
      throw "MSI must contain exactly one $Name property; found $($rows.Count)"
    }
    return [string]$rows[0].Value
  }

  $productName = Get-RequiredMsiProperty -Name 'ProductName'
  if ($productName -cne 'FyAgent') {
    throw "MSI ProductName must be FyAgent; found '$productName'"
  }

  $productVersion = Get-RequiredMsiProperty -Name 'ProductVersion'
  if ($productVersion -cne $AppVersion) {
    throw "MSI ProductVersion $productVersion does not match $AppVersion"
  }

  $arpNoRepair = Get-RequiredMsiProperty -Name 'ARPNOREPAIR'
  if ($arpNoRepair -cne '1' -and $arpNoRepair.ToLowerInvariant() -cne 'yes') {
    throw "MSI ARPNOREPAIR must be 1 or yes; found '$arpNoRepair'"
  }

  $registryRows = @(Get-MsiRows `
    -Query 'SELECT `Root`, `Key`, `Name`, `Value` FROM `Registry`' `
    -Columns @('Root', 'Key', 'Name', 'Value'))
  $protocolRoot = 'Software\Classes\fyagent'
  $hasProtocolMarker = $false
  $hasProtocolDefault = $false
  $hasProtocolIcon = $false
  $hasProtocolCommand = $false
  foreach ($row in $registryRows) {
    if (
      $row.Root -ceq '2' -and
      $row.Key -ceq $protocolRoot -and
      $row.Name -ceq 'URL Protocol' -and
      [string]::IsNullOrEmpty($row.Value)
    ) {
      $hasProtocolMarker = $true
    }
    if (
      $row.Root -ceq '2' -and
      $row.Key -ceq $protocolRoot -and
      [string]::IsNullOrEmpty($row.Name) -and
      $row.Value -ceq 'URL:FyAgent protocol'
    ) {
      $hasProtocolDefault = $true
    }
    if (
      $row.Root -ceq '2' -and
      $row.Key -ceq "$protocolRoot\DefaultIcon" -and
      [string]::IsNullOrEmpty($row.Name) -and
      $row.Value -ceq '"[!Path]",0'
    ) {
      $hasProtocolIcon = $true
    }
    if (
      $row.Root -ceq '2' -and
      $row.Key -ceq "$protocolRoot\shell\open\command" -and
      [string]::IsNullOrEmpty($row.Name) -and
      $row.Value -ceq '"[!Path]" "%1"'
    ) {
      $hasProtocolCommand = $true
    }
  }
  if (
    -not $hasProtocolMarker -or
    -not $hasProtocolDefault -or
    -not $hasProtocolIcon -or
    -not $hasProtocolCommand
  ) {
    throw 'MSI does not contain the complete fyagent URL protocol registry contract'
  }

  $fileRows = @(Get-MsiRows `
    -Query 'SELECT `File`, `FileName` FROM `File`' `
    -Columns @('File', 'FileName'))
  $fyAgentExecutableRows = [Collections.Generic.List[object]]::new()
  foreach ($row in $fileRows) {
    $longName = @($row.FileName -split '\|')[-1]
    if ($row.File -ceq 'Path' -and $longName -match '(?i)(?:^|-)fyagent\.exe$') {
      $fyAgentExecutableRows.Add($row)
    }
    if ($longName -match '(?i)^fyagent_installer_actions\.dll$') {
      throw 'MSI must not install the custom-action DLL as application payload'
    }
  }
  if ($fyAgentExecutableRows.Count -ne 1) {
    throw "MSI File table must contain exactly one Path payload ending in fyagent.exe; found $($fyAgentExecutableRows.Count)"
  }

  $binaryView = $null
  $binaryRecord = $null
  $binaryOutput = $null
  $embeddedHelper = Join-Path $temporaryRoot 'Binary.FyAgentInstallerActions'
  try {
    $binaryView = $database.OpenView(
      "SELECT ``Data`` FROM ``Binary`` WHERE ``Name``='FyAgentInstallerActions'"
    )
    $binaryView.Execute()
    $binaryRecord = $binaryView.Fetch()
    if ($null -eq $binaryRecord) {
      throw 'MSI does not contain Binary.FyAgentInstallerActions'
    }

    [int64]$streamSize = $binaryRecord.DataSize(1)
    if ($streamSize -le 0) {
      throw 'Binary.FyAgentInstallerActions is empty'
    }

    $latin1 = [Text.Encoding]::GetEncoding(28591)
    $binaryOutput = [IO.File]::Open(
      $embeddedHelper,
      [IO.FileMode]::CreateNew,
      [IO.FileAccess]::Write,
      [IO.FileShare]::None
    )
    [int64]$remaining = $streamSize
    while ($remaining -gt 0) {
      [int]$requested = [Math]::Min(1MB, $remaining)
      [string]$chunk = $binaryRecord.ReadStream(1, $requested, 1)
      if ([string]::IsNullOrEmpty($chunk)) {
        throw "Binary.FyAgentInstallerActions ended before its declared $streamSize bytes"
      }
      foreach ($character in $chunk.ToCharArray()) {
        if ([int]$character -gt 255) {
          throw 'Binary.FyAgentInstallerActions returned a non-byte character'
        }
      }
      [byte[]]$chunkBytes = $latin1.GetBytes($chunk)
      if ($chunkBytes.Length -ne $chunk.Length -or $chunkBytes.Length -gt $remaining) {
        throw 'Binary.FyAgentInstallerActions returned an invalid stream chunk'
      }
      $binaryOutput.Write($chunkBytes, 0, $chunkBytes.Length)
      $remaining -= $chunkBytes.Length
    }

    [string]$extra = $binaryRecord.ReadStream(1, 1, 1)
    if (-not [string]::IsNullOrEmpty($extra)) {
      throw 'Binary.FyAgentInstallerActions exceeds its declared stream size'
    }
  } finally {
    if ($null -ne $binaryOutput) {
      $binaryOutput.Dispose()
    }
    Release-ComObject $binaryRecord
    if ($null -ne $binaryView) {
      try {
        $binaryView.Close()
      } catch {
        Write-Warning "Failed to close the read-only MSI Binary view: $($_.Exception.Message)"
      }
    }
    Release-ComObject $binaryView
  }

  $embeddedLength = (Get-Item -LiteralPath $embeddedHelper -ErrorAction Stop).Length
  $helperLength = (Get-Item -LiteralPath $resolvedHelper -ErrorAction Stop).Length
  if ($embeddedLength -ne $helperLength) {
    throw "Embedded installer-actions length $embeddedLength differs from built helper length $helperLength"
  }

  $embeddedMachine = Get-PeMachine -Path $embeddedHelper -Description 'Embedded installer-actions DLL'
  if ($embeddedMachine -ne $expectedMachine) {
    throw "Embedded installer-actions DLL machine is 0x$($embeddedMachine.ToString('X4')); expected $expectedMachineName"
  }

  $embeddedSha256 = (Get-FileHash -LiteralPath $embeddedHelper -Algorithm SHA256).Hash
  $helperSha256 = (Get-FileHash -LiteralPath $resolvedHelper -Algorithm SHA256).Hash
  if ($embeddedSha256 -cne $helperSha256) {
    throw "Embedded installer-actions SHA-256 $embeddedSha256 differs from built helper SHA-256 $helperSha256"
  }

  $summaryInformation = $null
  try {
    $summaryInformation = $database.SummaryInformation(0)
    $summaryTemplate = [string]$summaryInformation.Property(7)
    $summaryArchitecture = @($summaryTemplate -split ';')[0]
    $expectedSummaryArchitecture = if ($Architecture -eq 'arm64') { 'Arm64' } else { 'x64' }
    if ($summaryArchitecture -cne $expectedSummaryArchitecture) {
      throw "MSI summary architecture $summaryArchitecture does not match $expectedSummaryArchitecture"
    }
  } finally {
    Release-ComObject $summaryInformation
  }

  # Export only string tables. Binary is read through ReadStream above so raw
  # DLL bytes cannot hide or create false-positive host-path evidence here.
  $tablesToScan = @(
    'Property',
    'Directory',
    'Component',
    'Feature',
    'FeatureComponents',
    'File',
    'Registry',
    'Shortcut',
    'Upgrade',
    'CustomAction',
    'InstallUISequence',
    'InstallExecuteSequence',
    'Dialog',
    'Control',
    'ControlEvent',
    'MsiLockPermissionsEx',
    'AppSearch',
    'RegLocator'
  )
  $forbiddenHostFragments = @(
    '/home/',
    '/workspace/',
    '/mnt/',
    '/projects/',
    'Z:\',
    'Z:/',
    '\\wsl$\',
    '\\wsl.localhost\',
    'cargo-xwin',
    'osxcross',
    'macos-cross',
    'windows-cross'
  )
  function Assert-NoCrossHostResidue {
    param(
      [Parameter(Mandatory = $true)][string]$Path,
      [Parameter(Mandatory = $true)][string]$Description
    )

    [byte[]]$bytes = [IO.File]::ReadAllBytes($Path)
    $textViews = @(
      [Text.Encoding]::GetEncoding(28591).GetString($bytes),
      [Text.Encoding]::Unicode.GetString($bytes)
    )
    foreach ($fragment in $forbiddenHostFragments) {
      foreach ($textView in $textViews) {
        if ($textView.IndexOf($fragment, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
          throw "$Description contains Linux or retired cross-build host-path residue: $fragment"
        }
      }
    }
  }

  Assert-NoCrossHostResidue `
    -Path $embeddedHelper `
    -Description 'MSI Binary.FyAgentInstallerActions'
  foreach ($table in $tablesToScan) {
    $exportName = "$table.idt"
    $database.Export($table, $temporaryRoot, $exportName)
    $exportPath = Join-Path $temporaryRoot $exportName
    Assert-NoCrossHostResidue -Path $exportPath -Description "MSI $table table"
  }

  Write-Host (
    "Verified FyAgent $AppVersion Windows $Architecture MSI: ProductName, ARPNOREPAIR, " +
    "fyagent protocol/payload, host-path boundary, and Binary.FyAgentInstallerActions " +
    "PE/SHA-256 binding"
  )
} finally {
  Release-ComObject $database
  Release-ComObject $installer
  if (Test-Path -LiteralPath $temporaryRoot) {
    Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
  }
}
