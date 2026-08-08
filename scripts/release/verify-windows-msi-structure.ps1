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
$maxDirectoryRows = 4096
$maxComponentRows = 32768
$maxMsiFieldUtf16Units = 1024

function Release-ComObject {
  param([object]$Value)

  if ($null -ne $Value -and [Runtime.InteropServices.Marshal]::IsComObject($Value)) {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Value)
  }
}

try {
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $database = $installer.OpenDatabase($resolvedMsi, 0)

  function Get-MsiRecord([string]$query) {
    $records = @(Get-MsiRecords -Query $query -MaxRows 2)
    if ($records.Count -gt 1) {
      throw "MSI query expected at most one row; found $($records.Count): $query"
    }
    if ($records.Count -eq 0) {
      return $null
    }
    return $records[0]
  }

  function Get-MsiRecords {
    param(
      [Parameter(Mandatory = $true)]
      [string]$Query,

      [int]$MaxRows = 32768
    )

    if ($MaxRows -lt 1) {
      throw "MSI query row cap must be positive: $MaxRows"
    }
    $rows = [Collections.Generic.List[object]]::new()
    $view = $null
    $record = $null
    try {
      $view = $database.OpenView($Query)
      [void]$view.Execute()
      while ($null -ne ($record = $view.Fetch())) {
        try {
          if ($rows.Count -ge $MaxRows) {
            throw "MSI query exceeds its $MaxRows-row cap: $Query"
          }
          [int]$fieldCount = $record.FieldCount
          if ($fieldCount -lt 1 -or $fieldCount -gt 16) {
            throw "MSI query returned an invalid field count ${fieldCount}: $Query"
          }
          $values = [string[]]::new($fieldCount)
          for ($field = 1; $field -le $fieldCount; $field += 1) {
            $value = [string]$record.StringData($field)
            if ($value.Length -gt $maxMsiFieldUtf16Units) {
              throw "MSI query field exceeds $maxMsiFieldUtf16Units UTF-16 units: $Query"
            }
            $values[$field - 1] = $value
          }
          [void]$rows.Add([PSCustomObject]@{ Values = $values })
        } finally {
          Release-ComObject $record
          $record = $null
        }
      }
    } finally {
      Release-ComObject $record
      if ($null -ne $view) {
        try {
          [void]$view.Close()
        } catch {
          Write-Warning "Failed to close a read-only MSI query view: $($_.Exception.Message)"
        }
      }
      Release-ComObject $view
    }
    return $rows.ToArray()
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
      [void]$view.Execute()
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
          [void]$view.Close()
        } catch {
          Write-Warning "Failed to close the read-only MSI stream view: $($_.Exception.Message)"
        }
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
      }
    }
  }

  function Get-MsiSequenceRow([string]$table, [string]$action) {
    $records = @(Get-MsiRecords "SELECT ``Condition``, ``Sequence`` FROM ``$table`` WHERE ``Action``='$action'")
    if ($records.Count -ne 1) {
      throw "MSI contract requires exactly one sequence action $action in $table; found $($records.Count)"
    }
    $record = $records[0]
    return [PSCustomObject]@{
      Action = $action
      Condition = [string]$record.Values[0]
      Sequence = [int]$record.Values[1]
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
    $records = @(Get-MsiRecords "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action``='$action'")
    if (
      $records.Count -ne 1 -or
      [int]$records[0].Values[0] -ne $expectedType -or
      [string]$records[0].Values[1] -cne $expectedSource -or
      [string]$records[0].Values[2] -cne $expectedTarget
    ) {
      throw "MSI custom action contract drifted for $action"
    }
  }

  function Assert-MsiPropertyValue([string]$property, [string]$expectedValue) {
    $records = @(Get-MsiRecords "SELECT ``Value`` FROM ``Property`` WHERE ``Property``='$property'")
    if (
      $records.Count -ne 1 -or
      [string]$records[0].Values[0] -cne $expectedValue
    ) {
      throw "MSI Property-table default drifted for $property"
    }
  }

  function Assert-PerMachineShortcut(
    [string]$shortcut,
    [string]$expectedDirectory
  ) {
    $shortcutRecord = Get-MsiRecord "SELECT ``Directory_``, ``Component_``, ``Target`` FROM ``Shortcut`` WHERE ``Shortcut``='$shortcut'"
    if (
      $null -eq $shortcutRecord -or
      [string]$shortcutRecord.Values[0] -cne $expectedDirectory -or
      [string]$shortcutRecord.Values[1] -cne 'Path' -or
      [string]::IsNullOrWhiteSpace([string]$shortcutRecord.Values[2])
    ) {
      throw "MSI per-machine shortcut row drifted for $shortcut"
    }
    $targetFeature = [string]$shortcutRecord.Values[2]
    if ($targetFeature -cnotmatch '^[A-Za-z_][A-Za-z0-9_.]{0,37}$') {
      throw "MSI shortcut $shortcut has an invalid advertised Feature identifier"
    }
    if ($null -eq (Get-MsiRecord "SELECT ``Feature`` FROM ``Feature`` WHERE ``Feature``='$targetFeature'")) {
      throw "MSI shortcut $shortcut is not authored as an advertised feature target"
    }
    Require-MsiRecord "SELECT ``Feature_`` FROM ``FeatureComponents`` WHERE ``Feature_``='$targetFeature' AND ``Component_``='Path'" "advertised shortcut $shortcut feature owns the Path component"
    return $targetFeature
  }

  function Assert-MsiControlEventCondition(
    [string]$event,
    [string]$argument,
    [string]$expectedCondition,
    [int]$expectedOrdering
  ) {
    $records = @(Get-MsiRecords "SELECT ``Condition``, ``Ordering`` FROM ``ControlEvent`` WHERE ``Dialog_``='InstallDirDlg' AND ``Control_``='Next' AND ``Event``='$event' AND ``Argument``='$argument'")
    if (
      $records.Count -ne 1 -or
      [string]$records[0].Values[0] -cne $expectedCondition -or
      [int]$records[0].Values[1] -ne $expectedOrdering
    ) {
      throw "MSI InstallDir Next condition drifted for $event/$argument"
    }
  }

  Require-MsiRecord 'SELECT `Name` FROM `Binary` WHERE `Name`=''FyAgentInstallerActions''' 'architecture-matched installer-actions Binary row'
  $installerActionPayloadRows = @(
    Get-MsiRecords 'SELECT `File`, `FileName` FROM `File`' | Where-Object {
      if ($null -eq $_) {
        $false
      } else {
        $payloadName = [string]$_.Values[1]
        $payloadName -match '(?i)(?:^|\|)fyagent_installer_actions\.dll$'
      }
    }
  )
  if ($installerActionPayloadRows.Count -ne 0) {
    throw 'MSI must not install the custom-action DLL as an application payload'
  }

  $productVersion = Get-MsiRecord 'SELECT `Value` FROM `Property` WHERE `Property`=''ProductVersion'''
  if ($null -eq $productVersion) {
    throw 'MSI Property table is missing ProductVersion'
  }
  $productVersionValue = [string]$productVersion.Values[0]
  if ($productVersionValue -ne $AppVersion) {
    throw "MSI ProductVersion does not match frozen APP_VERSION $AppVersion"
  }

  # Bind the exact built executable to the compressed MSI payload without
  # running the installer or any custom action. MSI cabinets store File-table
  # keys, so expand.exe extracts only the fixed `Path` token into a fresh root.
  $payloadRecord = Get-MsiRecord 'SELECT `FileName`, `FileSize`, `Sequence` FROM `File` WHERE `File`=''Path'''
  if ($null -eq $payloadRecord) {
    throw 'MSI File table is missing the unique Path executable payload'
  }
  $payloadLongName = @([string]$payloadRecord.Values[0] -split '\|')[-1]
  if ($payloadLongName -notmatch '(?i)(?:^|-)fyagent\.exe$') {
    throw "MSI Path payload is not fyagent.exe: $payloadLongName"
  }
  [int64]$payloadFileSize = $payloadRecord.Values[1]
  [int]$payloadSequence = $payloadRecord.Values[2]
  $builtSize = (Get-Item -LiteralPath $resolvedBuiltExe -ErrorAction Stop).Length
  if ($payloadFileSize -ne $builtSize) {
    throw "MSI File table size $payloadFileSize differs from built fyagent.exe size $builtSize"
  }

  $candidateMedia = @(
    @(
      foreach ($row in @(Get-MsiRecords 'SELECT `DiskId`, `LastSequence`, `Cabinet` FROM `Media`')) {
        if ([int]$row.Values[1] -ge $payloadSequence) {
          [PSCustomObject]@{
            DiskId = [int]$row.Values[0]
            LastSequence = [int]$row.Values[1]
            Cabinet = [string]$row.Values[2]
          }
        }
      }
    ) | Sort-Object LastSequence, DiskId
  )
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
  $expandCommand = Get-Command expand.exe -ErrorAction Stop
  $expandProcess = Start-Process -FilePath $expandCommand.Source -ArgumentList @(
    $cabinetPath,
    '-F:Path',
    $expandedRoot
  ) -NoNewWindow -Wait -PassThru
  if ($expandProcess.ExitCode -ne 0) {
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
    [int]$uiAction.Values[0] -ne 1 -or
    [string]$uiAction.Values[1] -cne 'FyAgentInstallerActions' -or
    [string]$uiAction.Values[2] -cne 'ValidateFyAgentInstallDirUi'
  ) {
    throw 'MSI UI directory action is not the expected Type 1 DLL entry'
  }

  $executeAction = Get-MsiRecord 'SELECT `Type`, `Source`, `Target` FROM `CustomAction` WHERE `Action`=''ValidateFyAgentInstallDirExecute'''
  if (
    $null -eq $executeAction -or
    [int]$executeAction.Values[0] -ne 1 -or
    [string]$executeAction.Values[1] -cne 'FyAgentInstallerActions' -or
    [string]$executeAction.Values[2] -cne 'ValidateFyAgentInstallDirExecute'
  ) {
    throw 'MSI execute directory action is not the expected Type 1 DLL entry'
  }

  Assert-MsiCustomAction 'ClearFyAgentPreviousInstallDir' 307 'FYAGENT_PREVIOUS_INSTALLDIR' ''
  Assert-MsiCustomAction 'ClearMaintenanceInstallDir' 307 'INSTALLDIR' ''
  Assert-MsiCustomAction 'EnforceFyAgentAllUsers' 51 'ALLUSERS' '1'
  Assert-MsiCustomAction 'EnforceFyAgentDisableAdvertisedShortcuts' 51 'DISABLEADVTSHORTCUTS' '1'
  Assert-MsiCustomAction 'RestoreInstallDirFromPrevious' 51 'INSTALLDIR' '[FYAGENT_PREVIOUS_INSTALLDIR]'
  Assert-MsiCustomAction 'ClassifyFyAgentPureUninstall' 1 'FyAgentInstallerActions' 'ClassifyFyAgentPureUninstall'
  Assert-MsiCustomAction 'ApplyValidatedFyAgentInstallDir' 35 'INSTALLDIR' '[FYAGENT_INSTALLDIR_NORMALIZED]'
  Assert-MsiCustomAction 'AbortUnsafeFyAgentInstallDir' 19 '' '[FYAGENT_INSTALLDIR_ERROR_MESSAGE]'
  foreach ($obsoleteAction in @('ClearFyAgentPureUninstall', 'SetFyAgentPureUninstall')) {
    if ($null -ne (Get-MsiRecord "SELECT ``Action`` FROM ``CustomAction`` WHERE ``Action``='$obsoleteAction'")) {
      throw "MSI still contains obsolete authored component-state action $obsoleteAction"
    }
  }

  $missingAnchorAction = Get-MsiRecord 'SELECT `Type` FROM `CustomAction` WHERE `Action`=''AbortUntrustedFyAgentMaintenance'''
  if ($null -eq $missingAnchorAction -or [int]$missingAnchorAction.Values[0] -ne 19) {
    throw 'MSI missing-InstallDir-anchor action is not Type 19'
  }

  Require-MsiRecord 'SELECT `Property` FROM `AppSearch` WHERE `Property`=''FYAGENT_PREVIOUS_INSTALLDIR'' AND `Signature_`=''FyAgentPreviousInstallDir''' 'HKLM InstallDir AppSearch contract'
  Require-MsiRecord 'SELECT `Signature_` FROM `RegLocator` WHERE `Signature_`=''FyAgentPreviousInstallDir'' AND `Root`=2 AND `Key`=''Software\fyagent\FyAgent'' AND `Name`=''InstallDir'' AND `Type`=18' 'HKLM InstallDir registry locator'
  $installDirLock = Get-MsiRecord 'SELECT `SDDLText` FROM `MsiLockPermissionsEx` WHERE `LockObject`=''INSTALLDIR'''
  if (
    $null -eq $installDirLock -or
    [string]$installDirLock.Values[0] -cne 'O:SYD:P(A;OICI;0x1200a9;;;BU)(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)'
  ) {
    throw 'MSI INSTALLDIR protected DACL contract drifted'
  }

  Assert-MsiPropertyValue 'ALLUSERS' '1'
  Assert-MsiPropertyValue 'DISABLEADVTSHORTCUTS' '1'
  if ($null -ne (Get-MsiRecord 'SELECT `Property` FROM `Property` WHERE `Property`=''FyAgentPureUninstall''')) {
    throw 'MSI must not author a default for the private pure-uninstall classifier marker'
  }
  if ($null -ne (Get-MsiRecord 'SELECT `Property` FROM `Property` WHERE `Property`=''FYAGENT_PURE_UNINSTALL''')) {
    throw 'MSI still contains the obsolete public pure-uninstall marker'
  }
  Require-MsiRecord 'SELECT `Directory` FROM `Directory` WHERE `Directory`=''DesktopFolder'' AND `Directory_Parent`=''TARGETDIR''' 'context-redirected DesktopFolder'
  Require-MsiRecord 'SELECT `Directory` FROM `Directory` WHERE `Directory`=''ProgramMenuFolder'' AND `Directory_Parent`=''TARGETDIR''' 'context-redirected ProgramMenuFolder'
  Require-MsiRecord 'SELECT `Directory` FROM `Directory` WHERE `Directory`=''ApplicationProgramsFolder'' AND `Directory_Parent`=''ProgramMenuFolder''' 'FyAgent product directory below ProgramMenuFolder'
  $pathComponent = Get-MsiRecord 'SELECT `Directory_`, `Attributes`, `KeyPath` FROM `Component` WHERE `Component`=''Path'''
  if (
    $null -eq $pathComponent -or
    [string]$pathComponent.Values[0] -cne 'INSTALLDIR' -or
    ([int]$pathComponent.Values[1] -band 4) -ne 0 -or
    [string]$pathComponent.Values[2] -cne 'Path'
  ) {
    throw 'MSI Path component must use the installed executable as its file KeyPath'
  }
  Require-MsiRecord 'SELECT `File` FROM `File` WHERE `File`=''Path'' AND `Component_`=''Path''' 'Path executable owned by the Path component'
  $desktopShortcutFeature = Assert-PerMachineShortcut 'ApplicationDesktopShortcut' 'DesktopFolder'
  $startMenuShortcutFeature = Assert-PerMachineShortcut 'ApplicationStartMenuShortcut' 'ApplicationProgramsFolder'
  if ($desktopShortcutFeature -cne $startMenuShortcutFeature) {
    throw 'MSI desktop and Start Menu shortcuts must target the same Path-owning feature'
  }
  if ($null -ne (Get-MsiRecord 'SELECT `Registry` FROM `Registry` WHERE `Name`=''DesktopShortcut'' OR `Name`=''StartMenuShortcut''')) {
    throw 'MSI still contains obsolete standalone shortcut marker values'
  }
  if ($null -ne (Get-MsiRecord 'SELECT `FileKey` FROM `RemoveFile` WHERE `DirProperty`=''DesktopFolder''')) {
    throw 'MSI must not attempt to remove the context-redirected DesktopFolder root'
  }
  if ($null -ne (Get-MsiRecord 'SELECT `FileKey` FROM `RemoveFile` WHERE `DirProperty`=''ProgramMenuFolder''')) {
    throw 'MSI must not attempt to remove the context-redirected ProgramMenuFolder root'
  }
  $programFolderCleanup = @(Get-MsiRecords 'SELECT `FileKey`, `Component_`, `FileName`, `InstallMode` FROM `RemoveFile` WHERE `DirProperty`=''ApplicationProgramsFolder''')
  if (
    $programFolderCleanup.Count -ne 1 -or
    [string]$programFolderCleanup[0].Values[0] -cne 'RemoveApplicationProgramsFolder' -or
    [string]$programFolderCleanup[0].Values[1] -cne 'Path' -or
    -not [string]::IsNullOrEmpty([string]$programFolderCleanup[0].Values[2]) -or
    [int]$programFolderCleanup[0].Values[3] -ne 2
  ) {
    throw 'MSI product Start Menu directory cleanup row drifted'
  }

  # Recompute every rendered component rooted at INSTALLDIR so a template
  # addition cannot make a mixed remove/add transaction skip admission.
  $directoryParents = [Collections.Generic.Dictionary[string, string]]::new(
    [StringComparer]::Ordinal
  )
  foreach ($row in @(Get-MsiRecords -Query 'SELECT `Directory`, `Directory_Parent` FROM `Directory`' -MaxRows $maxDirectoryRows)) {
    $directory = [string]$row.Values[0]
    $directoryParent = [string]$row.Values[1]
    if ([string]::IsNullOrWhiteSpace($directory)) {
      throw 'MSI Directory table contains an empty identifier'
    }
    if (
      -not [string]::IsNullOrEmpty($directoryParent) -and
      [string]::IsNullOrWhiteSpace($directoryParent)
    ) {
      throw "MSI Directory table contains a whitespace-only parent for $directory"
    }
    if ($directoryParents.ContainsKey($directory)) {
      throw "MSI Directory table contains duplicate identifier $directory"
    }
    $directoryParents.Add($directory, $directoryParent)
  }

  function Test-InstallDirDescendant([string]$directory) {
    $visited = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $belowInstallDir = $false
    while (-not [string]::IsNullOrWhiteSpace($directory)) {
      if ($directory -ceq 'INSTALLDIR') {
        $belowInstallDir = $true
      }
      if (-not $visited.Add($directory)) {
        throw "MSI Directory table contains a cycle through $directory"
      }
      if (-not $directoryParents.ContainsKey($directory)) {
        throw "MSI Component references unknown directory $directory"
      }
      $directory = $directoryParents[$directory]
    }
    return $belowInstallDir
  }

  if (-not $directoryParents.ContainsKey('INSTALLDIR')) {
    throw 'MSI Directory table is missing INSTALLDIR'
  }
  foreach ($directory in @($directoryParents.Keys)) {
    # Validate every rendered parent chain, including directories which do not
    # currently own components and INSTALLDIR's own ancestors.
    [void](Test-InstallDirDescendant $directory)
  }

  $requiredInstallDirComponents = @(
    'CMP_UninstallShortcut',
    'InstallDirectoryAcl',
    'Path',
    'RegistryEntries'
  )
  [Array]::Sort($requiredInstallDirComponents, [StringComparer]::Ordinal)
  $allComponentSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  $actualComponentSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  foreach ($row in @(Get-MsiRecords -Query 'SELECT `Component`, `Directory_` FROM `Component`' -MaxRows $maxComponentRows)) {
    $component = [string]$row.Values[0]
    $componentDirectory = [string]$row.Values[1]
    if (
      [string]::IsNullOrWhiteSpace($component) -or
      [string]::IsNullOrWhiteSpace($componentDirectory) -or
      -not $allComponentSet.Add($component)
    ) {
      throw "MSI Component table contains an empty or duplicate identifier/directory: $component"
    }
    if (Test-InstallDirDescendant $componentDirectory) {
      [void]$actualComponentSet.Add($component)
    }
  }
  $actualInstallDirComponents = @($actualComponentSet)
  [Array]::Sort($actualInstallDirComponents, [StringComparer]::Ordinal)
  if ($actualInstallDirComponents.Count -lt $requiredInstallDirComponents.Count) {
    throw 'MSI INSTALLDIR component closure is unexpectedly smaller than its required core'
  }
  foreach ($requiredComponent in $requiredInstallDirComponents) {
    if ($actualInstallDirComponents -cnotcontains $requiredComponent) {
      throw "MSI INSTALLDIR component closure is missing required core component $requiredComponent"
    }
  }

  $maintenanceCondition = 'Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE'
  $pureUninstallProperty = 'FyAgentPureUninstall'
  $firstInstallCondition = 'NOT Installed AND NOT WIX_UPGRADE_DETECTED AND NOT UPGRADINGPRODUCTCODE'
  $missingAnchorCondition = "(Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE) AND NOT FYAGENT_PREVIOUS_INSTALLDIR AND NOT $pureUninstallProperty"
  $restoreDirectoryCondition = '(Installed OR WIX_UPGRADE_DETECTED OR UPGRADINGPRODUCTCODE) AND FYAGENT_PREVIOUS_INSTALLDIR'
  $activeDirectoryCondition = "NOT $pureUninstallProperty"
  $allowedDirectoryCondition = "$firstInstallCondition AND $activeDirectoryCondition AND FYAGENT_INSTALLDIR_VALID = `"1`""
  $rejectedDirectoryCondition = "$activeDirectoryCondition AND FYAGENT_INSTALLDIR_VALID <> `"1`""

  foreach ($table in @('InstallUISequence', 'InstallExecuteSequence')) {
    foreach ($row in @(Get-MsiRecords "SELECT ``Action``, ``Condition`` FROM ``$table``")) {
      [string]$condition = $row.Values[1]
      if ($condition.Length -gt 255) {
        throw "MSI $table condition exceeds the 255-character table limit for $($row.Values[0])"
      }
    }
  }
  foreach ($row in @(Get-MsiRecords 'SELECT `Dialog_`, `Control_`, `Event`, `Argument`, `Condition` FROM `ControlEvent`')) {
    [string]$condition = $row.Values[4]
    if ($condition.Length -gt 255) {
      throw "MSI ControlEvent condition exceeds the 255-character table limit for $($row.Values[0])/$($row.Values[1])/$($row.Values[2])/$($row.Values[3])"
    }
  }

  foreach ($table in @('InstallUISequence', 'InstallExecuteSequence')) {
    $enforceAllUsers = Assert-MsiSequenceCondition $table 'EnforceFyAgentAllUsers' '1'
    $enforceOrdinaryShortcuts = Assert-MsiSequenceCondition $table 'EnforceFyAgentDisableAdvertisedShortcuts' '1'
    $clearAnchor = Assert-MsiSequenceCondition $table 'ClearFyAgentPreviousInstallDir' $maintenanceCondition
    $clearInstallDir = Assert-MsiSequenceCondition $table 'ClearMaintenanceInstallDir' $maintenanceCondition
    $classifier = Assert-MsiSequenceCondition $table 'ClassifyFyAgentPureUninstall' '1'
    $missingAnchor = Assert-MsiSequenceCondition $table 'AbortUntrustedFyAgentMaintenance' $missingAnchorCondition
    $restoreInstallDir = Assert-MsiSequenceCondition $table 'RestoreInstallDirFromPrevious' $restoreDirectoryCondition
    $costInitialize = Get-MsiSequenceRow $table 'CostInitialize'
    $appSearch = Get-MsiSequenceRow $table 'AppSearch'
    $costFinalize = Get-MsiSequenceRow $table 'CostFinalize'
    Assert-MsiSequenceBefore $enforceAllUsers $costInitialize "$table enforce ALLUSERS before CostInitialize"
    Assert-MsiSequenceBefore $enforceOrdinaryShortcuts $costInitialize "$table enforce DISABLEADVTSHORTCUTS before CostInitialize"
    Assert-MsiSequenceBefore $clearAnchor $appSearch "$table clear anchor before AppSearch"
    Assert-MsiSequenceBefore $clearInstallDir $appSearch "$table clear INSTALLDIR before AppSearch"
    Assert-MsiSequenceBefore $appSearch $restoreInstallDir "$table AppSearch before InstallDir restore"
    Assert-MsiSequenceBefore $restoreInstallDir $costFinalize "$table InstallDir restore before CostFinalize"
    Assert-MsiSequenceBefore $costFinalize $classifier "$table CostFinalize before native pure-uninstall classifier"
    Assert-MsiSequenceBefore $classifier $missingAnchor "$table pure-uninstall classifier before missing-anchor abort"
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

  $removeShortcuts = Get-MsiSequenceRow 'InstallExecuteSequence' 'RemoveShortcuts'
  $removeFiles = Get-MsiSequenceRow 'InstallExecuteSequence' 'RemoveFiles'
  Assert-MsiSequenceBefore $removeShortcuts $removeFiles 'InstallExecuteSequence RemoveShortcuts before RemoveFiles'

  Require-MsiRecord 'SELECT `Dialog` FROM `Dialog` WHERE `Dialog`=''FyAgentUnsafeInstallDirDlg''' 'unsafe-directory dialog'
  $standardPathAccepted = '(WIXUI_DONTVALIDATEPATH OR WIXUI_INSTALLDIR_VALID="1")'
  Assert-MsiControlEventCondition 'SetTargetPath' '[WIXUI_INSTALLDIR]' '1' 1
  Assert-MsiControlEventCondition 'DoAction' 'WixUIValidatePath' 'NOT WIXUI_DONTVALIDATEPATH' 2
  Assert-MsiControlEventCondition 'SpawnDialog' 'InvalidDirDlg' 'NOT WIXUI_DONTVALIDATEPATH AND WIXUI_INSTALLDIR_VALID<>"1"' 3
  Assert-MsiControlEventCondition 'DoAction' 'ValidateFyAgentInstallDirUi' "$standardPathAccepted AND NOT FyAgentPureUninstall" 4
  Assert-MsiControlEventCondition 'DoAction' 'ApplyValidatedFyAgentInstallDir' "$standardPathAccepted AND NOT Installed AND NOT WIX_UPGRADE_DETECTED AND NOT UPGRADINGPRODUCTCODE AND FYAGENT_INSTALLDIR_VALID=`"1`" AND NOT FyAgentPureUninstall" 5
  Assert-MsiControlEventCondition 'SpawnDialog' 'FyAgentUnsafeInstallDirDlg' "$standardPathAccepted AND FYAGENT_INSTALLDIR_VALID<>`"1`" AND NOT FyAgentPureUninstall" 6
  Assert-MsiControlEventCondition 'NewDialog' 'VerifyReadyDlg' "$standardPathAccepted AND FYAGENT_INSTALLDIR_VALID=`"1`" AND NOT FyAgentPureUninstall" 7
  if ($null -ne (Get-MsiRecord 'SELECT `Action` FROM `CustomAction` WHERE `Action`=''ValidateInstallDirectory''')) {
    throw 'MSI still contains the legacy scripted directory action'
  }

  $template = [string]$database.SummaryInformation(0).Property(7)
  if ($template -notmatch "(?i)$Architecture") {
    throw "MSI summary template does not match ${Architecture}: $template"
  }
  Write-Output "Windows MSI structure OK: architecture=$Architecture version=$AppVersion INSTALLDIR-components=$($actualInstallDirComponents -join ',')"
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
