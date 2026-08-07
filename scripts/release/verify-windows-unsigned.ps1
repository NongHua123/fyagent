param(
  [Parameter(Mandatory = $true)]
  [string]$ExePath,

  [Parameter(Mandatory = $true)]
  [string]$MsiPath
)

$ErrorActionPreference = 'Stop'

foreach ($path in @($ExePath, $MsiPath)) {
  $resolved = (Resolve-Path -LiteralPath $path).Path
  $signature = Get-AuthenticodeSignature -FilePath $resolved
  if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
    throw "FyAgent v0.3.0 Windows artifacts must be unsigned; ${resolved} reported $($signature.Status)"
  }
  if ($null -ne $signature.SignerCertificate -or $null -ne $signature.TimeStamperCertificate) {
    throw "Unsigned Windows artifact unexpectedly exposes signer or timestamp certificates: ${resolved}"
  }
}
