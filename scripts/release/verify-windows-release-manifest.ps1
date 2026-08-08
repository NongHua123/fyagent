param(
  [Parameter(Mandatory = $true)]
  [string]$ExePath,

  [Parameter(Mandatory = $true)]
  [string]$AppVersion,

  [Parameter(Mandatory = $true)]
  [ValidateSet('x64', 'arm64')]
  [string]$Architecture,

  [Parameter(Mandatory = $true)]
  [string]$Phase
)

$ErrorActionPreference = 'Stop'

function Resolve-WindowsSdkManifestTool {
  param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('x64', 'arm64')]
    [string]$TargetArchitecture
  )

  $sdkArchitecture = if ($TargetArchitecture -eq 'arm64') { 'arm64' } else { 'x64' }
  $programRoots = @(
    [Environment]::GetEnvironmentVariable('ProgramFiles(x86)'),
    [Environment]::GetEnvironmentVariable('ProgramFiles')
  ) |
    Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
    Sort-Object -Unique

  $candidates = @(
    foreach ($programRoot in $programRoots) {
      $sdkBinRoot = Join-Path $programRoot 'Windows Kits\10\bin'
      if (-not (Test-Path -LiteralPath $sdkBinRoot -PathType Container)) {
        continue
      }

      foreach ($versionDirectory in Get-ChildItem -LiteralPath $sdkBinRoot -Directory) {
        $sdkVersion = $null
        if (-not [Version]::TryParse($versionDirectory.Name, [ref]$sdkVersion)) {
          continue
        }
        $toolPath = Join-Path $versionDirectory.FullName "$sdkArchitecture\mt.exe"
        if (Test-Path -LiteralPath $toolPath -PathType Leaf) {
          [PSCustomObject]@{
            Version = $sdkVersion
            Path = (Resolve-Path -LiteralPath $toolPath).Path
          }
        }
      }
    }
  )

  $selected = $candidates |
    Sort-Object -Property @{ Expression = 'Version'; Descending = $true }, @{ Expression = 'Path'; Descending = $false } |
    Select-Object -First 1
  if ($null -eq $selected) {
    throw "Architecture-matched Windows SDK mt.exe was not found for $TargetArchitecture"
  }
  return [string]$selected.Path
}

$resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path
$bytes = [IO.File]::ReadAllBytes($resolvedExe)
if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
  throw "Windows release executable is not a PE image: $resolvedExe"
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
  throw "Windows release executable has no valid PE header: $resolvedExe"
}
[uint16]$machine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
[uint16]$expectedMachine = if ($Architecture -eq 'arm64') { 0xAA64 } else { 0x8664 }
if ($machine -ne $expectedMachine) {
  throw "Windows release executable PE Machine is 0x$($machine.ToString('X4')); expected $Architecture (0x$($expectedMachine.ToString('X4')))"
}
$versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($resolvedExe)
if (
  [string]::IsNullOrWhiteSpace($versionInfo.ProductVersion) -or
  -not $versionInfo.ProductVersion.StartsWith($AppVersion, [StringComparison]::Ordinal)
) {
  throw "Windows executable ProductVersion does not match frozen APP_VERSION ${AppVersion}: $($versionInfo.ProductVersion)"
}

$safePhase = $Phase -replace '[^A-Za-z0-9_.-]', '-'
$manifestPath = Join-Path $env:RUNNER_TEMP "fyagent-release-${safePhase}.manifest"
Remove-Item -LiteralPath $manifestPath -Force -ErrorAction SilentlyContinue
$mtPath = Resolve-WindowsSdkManifestTool -TargetArchitecture $Architecture
Write-Host "Using Windows SDK Manifest Tool: $mtPath"

# mt.exe only reads the PE RT_MANIFEST resource; this verifier never executes fyagent.exe.
& $mtPath "-inputresource:$resolvedExe;#1" "-out:$manifestPath" -nologo
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
  throw "mt.exe did not extract RT_MANIFEST from ${resolvedExe} during ${Phase}"
}

$manifest = [System.Xml.XmlDocument]::new()
try {
  $manifest.Load($manifestPath)
} catch {
  throw "Extracted RT_MANIFEST is not valid XML: $($_.Exception.Message)"
}

$requestedExecutionLevels = @(
  $manifest.SelectNodes("//*[local-name()='requestedExecutionLevel']")
)
if ($requestedExecutionLevels.Count -ne 1) {
  throw "Embedded RT_MANIFEST must contain exactly one requestedExecutionLevel; found $($requestedExecutionLevels.Count)"
}

[System.Xml.XmlElement]$requestedExecutionLevel = $requestedExecutionLevels[0]
if (
  $requestedExecutionLevel.NamespaceURI -cne 'urn:schemas-microsoft-com:asm.v3' -or
  $requestedExecutionLevel.Attributes.Count -ne 2 -or
  -not $requestedExecutionLevel.HasAttribute('level') -or
  -not $requestedExecutionLevel.HasAttribute('uiAccess') -or
  $requestedExecutionLevel.GetAttribute('level') -cne 'requireAdministrator' -or
  $requestedExecutionLevel.GetAttribute('uiAccess') -cne 'false'
) {
  throw 'Embedded RT_MANIFEST must contain exactly one requestedExecutionLevel level="requireAdministrator" uiAccess="false"'
}
