param(
    [Parameter(Mandatory = $true)]
    [string] $Path
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'windows-signature-policy.ps1')

Assert-CurtPmeAuthenticodeSignature -FilePath $Path
Write-Host "CurtPME Windows signature verification passed: $Path"
