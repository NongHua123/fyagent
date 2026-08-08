param(
  [Parameter(Mandatory = $true)]
  [string]$MsiPath,

  [Parameter(Mandatory = $true)]
  [ValidateSet('x64', 'arm64')]
  [string]$Architecture,

  [Parameter(Mandatory = $true)]
  [string]$AppVersion,

  [Parameter(Mandatory = $true)]
  [string]$BuiltExePath
)

$ErrorActionPreference = 'Stop'
$resolvedMsi = (Resolve-Path -LiteralPath $MsiPath).Path
$resolvedBuiltExe = (Resolve-Path -LiteralPath $BuiltExePath).Path
$payloadRoot = Join-Path $env:RUNNER_TEMP "fyagent-msi-payload-$([Guid]::NewGuid().ToString('N'))"
$installer = $null
$database = $null

try {
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $database = $installer.OpenDatabase($resolvedMsi, 0)

  function Get-MsiRecord([string]$query) {
    $view = $database.OpenView($query)
    $view.Execute()
    return $view.Fetch()
  }

  function Get-MsiRecords([string]$query) {
    $view = $database.OpenView($query)
    $view.Execute()
    while ($record = $view.Fetch()) {
      $record
    }
  }

  function Require-MsiRecord([string]$query, [string]$description) {
    if ($null -eq (Get-MsiRecord $query)) {
      throw "MSI contract is missing: $description"
    }
  }

  function Get-PeMachine([string]$path, [string]$description) {
    [byte[]]$bytes = [IO.File]::ReadAllBytes($path)
    if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
      throw "$description is not a PE image"
    }
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
    if (
      $peOffset -lt 0x40 -or
      $peOffset + 6 -gt $bytes.Length -or
      $bytes[$peOffset] -ne 0x50 -or
      $bytes[$peOffset + 1] -ne 0x45 -or
      $bytes[$peOffset + 2] -ne 0 -or
      $bytes[$peOffset + 3] -ne 0
    ) {
      throw "$description has no valid PE header"
    }
    return [BitConverter]::ToUInt16($bytes, $peOffset + 4)
  }

  function Export-MsiStream([string]$streamName, [string]$outputPath) {
    if ($streamName -notmatch '^[A-Za-z0-9_.-]+$') {
      throw "MSI embedded cabinet stream name is unsafe: $streamName"
    }
    $view = $null
    $record = $null
    $output = $null
    try {
      $view = $database.OpenView(
        "SELECT ``Data`` FROM ``_Streams`` WHERE ``Name``='$streamName'"
      )
      $view.Execute()
      $record = $view.Fetch()
      if ($null -eq $record) {
        throw "MSI is missing embedded cabinet stream $streamName"
      }
      [int64]$streamSize = $record.DataSize(1)
      if ($streamSize -le 0) {
        throw "MSI embedded cabinet stream $streamName is empty"
      }
      $latin1 = [Text.Encoding]::GetEncoding(28591)
      $output = [IO.File]::Open(
        $outputPath,
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write,
        [IO.FileShare]::None
      )
      [int64]$remaining = $streamSize
      while ($remaining -gt 0) {
        [int]$requested = [Math]::Min(1MB, $remaining)
        [string]$chunk = $record.ReadStream(1, $requested, 1)
        if ([string]::IsNullOrEmpty($chunk)) {
          throw "MSI cabinet stream $streamName ended before $streamSize bytes"
        }
        foreach ($character in $chunk.ToCharArray()) {
          if ([int]$character -gt 255) {
            throw "MSI cabinet stream $streamName returned a non-byte character"
          }
        }
        [byte[]]$chunkBytes = $latin1.GetBytes($chunk)
        if ($chunkBytes.Length -ne $chunk.Length -or $chunkBytes.Length -gt $remaining) {
          throw "MSI cabinet stream $streamName returned an invalid chunk"
        }
        $output.Write($chunkBytes, 0, $chunkBytes.Length)
        $remaining -= $chunkBytes.Length
      }
      [string]$extra = $record.ReadStream(1, 1, 1)
      if (-not [string]::IsNullOrEmpty($extra)) {
        throw "MSI cabinet stream $streamName exceeds its declared size"
      }
    } finally {
      if ($null -ne $output) {
        $output.Dispose()
      }
      if ($null -ne $record) {
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record)
      }
      if ($null -ne $view) {
        try {
          $view.Close()
        } catch {
          Write-Warning "Failed to close the read-only MSI stream view: $($_.Exception.Message)"
        }
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
      }
    }
  }

  function Get-MsiSequenceRow([string]$table, [string]$action) {
    $record = Get-MsiRecord "SELECT ``Condition``, ``Sequence`` FROM ``$table`` WHERE ``Action``='$action'"
    if ($null -eq $record) {
      throw "MSI contract is missing sequence action $action in $table"
    }
    return [PSCustomObject]@{
      Action = $action
      Condition = [string]$record.StringData(1)
      Sequence = [int]$record.IntegerData(2)
    }
  }

  function Assert-MsiSequenceCondition([string]$table, [string]$action, [string]$expectedCondition) {
    $row = Get-MsiSequenceRow $table $action
    if ($row.Condition -cne $expectedCondition) {
      throw "MSI $table condition drifted for ${action}: '$($row.Condition)'"
    }
    return $row
  }

  function Assert-MsiSequenceBefore($earlier, $later, [string]$description) {
    if ($earlier.Sequence -ge $later.Sequence) {
      throw "MSI sequence order failed: $description ($($earlier.Sequence) >= $($later.Sequence))"
    }
  }

  function Assert-MsiCustomAction([string]$action, [int]$expectedType, [string]$expectedSource, [string]$expectedTarget) {
    $record = Get-MsiRecord "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action``='$action'"
    if (
      $null -eq $record -or
      $record.IntegerData(1) -ne $expectedType -or
      [string]$record.StringData(2) -cne $expectedSource -or
      [string]$record.StringData(3) -cne $expectedTarget
    ) {
      throw "MSI custom action contract drifted for $action"
    }
    return $record
  }

  Require-MsiRecord 'SELECT `Name` FROM `Binary` WHERE `Name`=''FyAgentInstallerActions''' 'architecture-matched installer-actions Binary row'
  if ($null -ne (Get-MsiRecord 'SELECT `File` FROM `File` WHERE `FileName` LIKE ''%fyagent_installer_actions.dll%''')) {
    throw 'MSI must not install the custom-action DLL as an application payload'
  }

  $productVersion = Get-MsiRecord 'SELECT `Value` FROM `Property` WHERE `Property`=''ProductVersion'''
  if ($null -eq $productVersion -or $productVersion.StringData(1) -ne $AppVersion) {
    throw "MSI ProductVersion does not match frozen APP_VERSION $AppVersion"
  }

  # Bind the exact built executable to the compressed MSI payload without
  # running the installer or any custom action. MSI cabinets store File-table
  # keys, so expand.exe extracts only the fixed `Path` token into a fresh root.
  $payloadRecord = Get-MsiRecord 'SELECT `FileName`, `FileSize`, `Sequence` FROM `File` WHERE `File`=''Path'''
  if ($null -eq $payloadRecord) {
    throw 'MSI File table is missing the unique Path executable payload'
  }
  $payloadLongName = @([string]$payloadRecord.StringData(1) -split '\|')[-1]
  if ($payloadLongName -notmatch '(?i)(?:^|-)fyagent\.exe$') {
    throw "MSI Path payload is not fyagent.exe: $payloadLongName"
  }
  [int64]$payloadFileSize = $payloadRecord.IntegerData(2)
  [int]$payloadSequence = $payloadRecord.IntegerData(3)
  $builtSize = (Get-Item -LiteralPath $resolvedBuiltExe -ErrorAction Stop).Length
  if ($payloadFileSize -ne $builtSize) {
    throw "MSI File table size $payloadFileSize differs from built fyagent.exe size $builtSize"
  }

  $candidateMedia = @(
    foreach ($row in @(Get-MsiRecords 'SELECT `DiskId`, `LastSequence`, `Cabinet` FROM `Media`')) {
      if ($row.IntegerData(2) -ge $payloadSequence) {
        [PSCustomObject]@{
          DiskId = [int]$row.IntegerData(1)
          LastSequence = [int]$row.IntegerData(2)
          Cabinet = [string]$row.StringData(3)
        }
      }
    }
  ) | Sort-Object LastSequence, DiskId
  if ($candidateMedia.Count -lt 1) {
    throw "MSI Media table does not cover executable sequence $payloadSequence"
  }
  $payloadMedia = $candidateMedia[0]
  if ($payloadMedia.Cabinet -notmatch '^#([A-Za-z0-9_.-]+)$') {
    throw "MSI executable must use one safe embedded cabinet; received $($payloadMedia.Cabinet)"
  }
  $cabinetStream = $Matches[1]

  New-Item -ItemType Directory -Path $payloadRoot -ErrorAction Stop | Out-Null
  $cabinetPath = Join-Path $payloadRoot 'payload.cab'
  $expandedRoot = Join-Path $payloadRoot 'expanded'
  New-Item -ItemType Directory -Path $expandedRoot -ErrorAction Stop | Out-Null
  Export-MsiStream -streamName $cabinetStream -outputPath $cabinetPath
  Get-Command expand.exe -ErrorAction Stop | Out-Null
  & expand.exe $cabinetPath '-F:Path' $expandedRoot
  if ($LASTEXITCODE -ne 0) {
    throw "expand.exe could not extract the fixed Path payload from $cabinetStream"
  }
  $expandedItems = @(Get-ChildItem -LiteralPath $expandedRoot -Force -Recurse)
  if (
    $expandedItems.Count -ne 1 -or
    $expandedItems[0].PSIsContainer -or
    ($expandedItems[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
  ) {
    throw 'MSI cabinet extraction must produce exactly one regular non-reparse Path payload'
  }
  $expandedExe = $expandedItems[0].FullName
  $resolvedExpandedRoot = (Resolve-Path -LiteralPath $expandedRoot).Path.TrimEnd('\')
  $resolvedExpandedExe = (Resolve-Path -LiteralPath $expandedExe).Path
  if (-not $resolvedExpandedExe.StartsWith(
      "$resolvedExpandedRoot\",
      [StringComparison]::OrdinalIgnoreCase
    )) {
    throw 'MSI cabinet extraction escaped the fresh payload root'
  }
  $expectedMachine = if ($Architecture -eq 'arm64') { 0xAA64 } else { 0x8664 }
  if ((Get-PeMachine $resolvedExpandedExe 'Extracted MSI fyagent.exe') -ne $expectedMachine) {
    throw "Extracted MSI fyagent.exe PE Machine does not match $Architecture"
  }
  $builtSha256 = (Get-FileHash -LiteralPath $resolvedBuiltExe -Algorithm SHA256).Hash
  $expandedSha256 = (Get-FileHash -LiteralPath $resolvedExpandedExe -Algorithm SHA256).Hash
  if ($expandedSha256 -cne $builtSha256) {
    throw 'Extracted MSI fyagent.exe SHA-256 differs from the verified built executable'
  }
  $expandedSignature = Get-AuthenticodeSignature -FilePath $resolvedExpandedExe
  if (
    $expandedSignature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned -or
    $null -ne $expandedSignature.SignerCertificate -or
    $null -ne $expandedSignature.TimeStamperCertificate
  ) {
    throw 'Extracted MSI fyagent.exe must remain Authenticode NotSigned'
  }

  $uiAction = Get-MsiRecord 'SELECT `Type`, `Source`, `Target` FROM `CustomAction` WHERE `Action`=''ValidateFyAgentInstallDirUi'''
  if (
    $null -eq $uiAction -or
    $uiAction.IntegerData(1) -ne 1 -or
    $uiAction.StringData(2) -ne 'FyAgentInstallerActions' -or
    $uiAction.StringData(3) -ne 'ValidateFyAgentInstallDirUi'
  ) {
    throw 'MSI UI directory action is not the expected Type 1 DLL entry'
  }

  $executeAction = Get-MsiRecord 'SELECT `Type`, `Source`, `Target` FROM `CustomAction` WHERE `Action`=''ValidateFyAgentInstallDirExecute'''
  if (
    $null -eq $executeAction -or
    $executeAction.IntegerData(1) -ne 1 -or
    $executeAction.StringData(2) -ne 'FyAgentInstallerActions' -or
    $executeAction.StringData(3) -ne 'ValidateFyAgentInstallDirExecute'
  ) {
    throw 'MSI execute directory action is not the expected Type 1 DLL entry'
  }

  Assert-MsiCustomAction 'ClearFyAgentPreviousInstallDir' 307 'FYAGENT_PREVIOUS_INSTALLDIR' ''
  Assert-MsiCustomAction 'ClearMaintenanceInstallDir' 307 'INSTALLDIR' ''
  Assert-MsiCustomAction 'RestoreInstallDirFromPrevious' 51 'INSTALLDIR' '[FYAGENT_PREVIOUS_INSTALLDIR]'
  Assert-MsiCustomAction 'AbortUnsafeFyAgentInstallDir' 19 '' '[FYAGENT_INSTALLDIR_ERROR_MESSAGE]'

  $missingAnchorAction = Get-MsiRecord 'SELECT `Type` FROM `CustomAction` WHERE `Action`=''AbortUntrustedFyAgentMaintenance'''
  if ($null -eq $missingAnchorAction -or $missingAnchorAction.IntegerData(1) -ne 19) {
    throw 'MSI missing-InstallDir-anchor action is not Type 19'
  }

  Require-MsiRecord 'SELECT `Property` FROM `AppSearch` WHERE `Property`=''FYAGENT_PREVIOUS_INSTALLDIR'' AND `Signature_`=''FyAgentPreviousInstallDir''' 'HKLM InstallDir AppSearch contract'
  Require-MsiRecord 'SELECT `Signature_` FROM `RegLocator` WHERE `Signature_`=''FyAgentPreviousInstallDir'' AND `Root`=2 AND `Key`=''Software\fyagent\FyAgent'' AND `Name`=''InstallDir'' AND `Type`=18' 'HKLM InstallDir registry locator'
  $installDirLock = Get-MsiRecord 'SELECT `SDDLText` FROM `MsiLockPermissionsEx` WHERE `LockObject`=''INSTALLDIR'''
  if (
    $null -eq $installDirLock -or
    $installDirLock.StringData(1) -cne 'O:SYD:P(A;OICI;0x1200a9;;;BU)(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)'
  ) {
    throw 'MSI INSTALLDIR protected DACL contract drifted'
  }

  # Recompute every rendered component rooted at INSTALLDIR so a template
  # addition cannot make a mixed remove/add transaction skip admission.
  $directoryParents = @{}
  foreach ($row in @(Get-MsiRecords 'SELECT `Directory`, `Directory_Parent` FROM `Directory`')) {
    $directoryParents[[string]$row.StringData(1)] = [string]$row.StringData(2)
  }

  function Test-InstallDirDescendant([string]$directory) {
    $visited = @{}
    while (-not [string]::IsNullOrWhiteSpace($directory) -and -not $visited.ContainsKey($directory)) {
      if ($directory -eq 'INSTALLDIR') {
        return $true
      }
      $visited[$directory] = $true
      if (-not $directoryParents.ContainsKey($directory)) {
        return $false
      }
      $directory = $directoryParents[$directory]
    }
    return $false
  }

  $expectedInstallDirComponents = @(
    'CMP_UninstallShortcut',
    'InstallDirectoryAcl',
    'Path',
    'RegistryEntries'
  )
  $actualInstallDirComponents = @(
    foreach ($row in @(Get-MsiRecords 'SELECT `Component`, `Directory_` FROM `Component`')) {
      if (Test-InstallDirDescendant ([string]$row.StringData(2))) {
        [string]$row.StringData(1)
      }
    }
  ) | Sort-Object
  $componentDifference = @(
    Compare-Object -ReferenceObject $expectedInstallDirComponents -DifferenceObject $actualInstallDirComponents
  )
  if ($componentDifference.Count -ne 0) {
    throw "MSI INSTALLDIR component guard drifted: $($componentDifference | Out-String)"
  }

  $maintenanceCondition = 'Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE'
  $pureUninstallCondition = '$CMP_UninstallShortcut = 2 AND $InstallDirectoryAcl = 2 AND $Path = 2 AND $RegistryEntries = 2'
  $missingAnchorCondition = "(Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE) AND NOT FYAGENT_PREVIOUS_INSTALLDIR AND NOT ($pureUninstallCondition)"
  $restoreDirectoryCondition = '(Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE) AND FYAGENT_PREVIOUS_INSTALLDIR'
  $activeDirectoryCondition = "NOT ($pureUninstallCondition)"
  $allowedDirectoryCondition = "$activeDirectoryCondition AND FYAGENT_INSTALLDIR_VALID = `"1`""
  $rejectedDirectoryCondition = "$activeDirectoryCondition AND FYAGENT_INSTALLDIR_VALID <> `"1`""

  foreach ($table in @('InstallUISequence', 'InstallExecuteSequence')) {
    $clearAnchor = Assert-MsiSequenceCondition $table 'ClearFyAgentPreviousInstallDir' $maintenanceCondition
    $clearInstallDir = Assert-MsiSequenceCondition $table 'ClearMaintenanceInstallDir' $maintenanceCondition
    $missingAnchor = Assert-MsiSequenceCondition $table 'AbortUntrustedFyAgentMaintenance' $missingAnchorCondition
    $restoreInstallDir = Assert-MsiSequenceCondition $table 'RestoreInstallDirFromPrevious' $restoreDirectoryCondition
    $appSearch = Get-MsiSequenceRow $table 'AppSearch'
    $costFinalize = Get-MsiSequenceRow $table 'CostFinalize'
    Assert-MsiSequenceBefore $clearAnchor $appSearch "$table clear anchor before AppSearch"
    Assert-MsiSequenceBefore $clearInstallDir $appSearch "$table clear INSTALLDIR before AppSearch"
    Assert-MsiSequenceBefore $appSearch $restoreInstallDir "$table AppSearch before InstallDir restore"
    Assert-MsiSequenceBefore $restoreInstallDir $costFinalize "$table InstallDir restore before CostFinalize"
    Assert-MsiSequenceBefore $costFinalize $missingAnchor "$table CostFinalize before missing-anchor abort"
    if ($table -eq 'InstallUISequence') {
      $validator = Assert-MsiSequenceCondition $table 'ValidateFyAgentInstallDirUi' $activeDirectoryCondition
    } else {
      $validator = Assert-MsiSequenceCondition $table 'ValidateFyAgentInstallDirExecute' $activeDirectoryCondition
    }
    $apply = Assert-MsiSequenceCondition $table 'ApplyValidatedFyAgentInstallDir' $allowedDirectoryCondition
    Assert-MsiSequenceBefore $missingAnchor $validator "$table missing-anchor abort before validator"
    Assert-MsiSequenceBefore $validator $apply "$table validator before normalized-directory apply"
    if ($table -eq 'InstallExecuteSequence') {
      $reject = Assert-MsiSequenceCondition $table 'AbortUnsafeFyAgentInstallDir' $rejectedDirectoryCondition
      $installValidate = Get-MsiSequenceRow $table 'InstallValidate'
      $installFiles = Get-MsiSequenceRow $table 'InstallFiles'
      Assert-MsiSequenceBefore $apply $reject "$table normalized-directory apply before unsafe-directory abort"
      Assert-MsiSequenceBefore $reject $installValidate "$table unsafe-directory abort before InstallValidate"
      Assert-MsiSequenceBefore $installValidate $installFiles "$table InstallValidate before InstallFiles"
    }
  }

  Require-MsiRecord 'SELECT `Dialog` FROM `Dialog` WHERE `Dialog`=''FyAgentUnsafeInstallDirDlg''' 'unsafe-directory dialog'
  Require-MsiRecord 'SELECT `Event` FROM `ControlEvent` WHERE `Dialog_`=''InstallDirDlg'' AND `Control_`=''Next'' AND `Event`=''DoAction'' AND `Argument`=''ValidateFyAgentInstallDirUi''' 'InstallDir Next native validator event'
  Require-MsiRecord 'SELECT `Event` FROM `ControlEvent` WHERE `Dialog_`=''InstallDirDlg'' AND `Control_`=''Next'' AND `Event`=''SpawnDialog'' AND `Argument`=''FyAgentUnsafeInstallDirDlg''' 'InstallDir unsafe-directory dialog event'
  if ($null -ne (Get-MsiRecord 'SELECT `Action` FROM `CustomAction` WHERE `Action`=''ValidateInstallDirectory''')) {
    throw 'MSI still contains the legacy scripted directory action'
  }

  $template = [string]$database.SummaryInformation(0).Property(7)
  if ($template -notmatch "(?i)$Architecture") {
    throw "MSI summary template does not match ${Architecture}: $template"
  }
} finally {
  if ($null -ne $database) {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($database)
  }
  if ($null -ne $installer) {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
  }
  if (Test-Path -LiteralPath $payloadRoot) {
    Remove-Item -LiteralPath $payloadRoot -Recurse -Force
  }
  [GC]::Collect()
  [GC]::WaitForPendingFinalizers()
}
