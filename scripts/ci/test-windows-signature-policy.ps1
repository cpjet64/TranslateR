param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'windows-signature-policy.ps1')

function New-TestSignature {
    param(
        [string] $Status = 'Valid',
        [string] $Subject = 'CN=Curt P. Software, O=Curt P. Software',
        [string] $TimestampSubject = 'CN=DigiCert Timestamp Responder, O=DigiCert Inc',
        [switch] $MissingSigner,
        [switch] $MissingTimestamp
    )

    [pscustomobject] @{
        Status = $Status
        SignerCertificate = if ($MissingSigner) { $null } else { [pscustomobject] @{ Subject = $Subject } }
        TimeStamperCertificate = if ($MissingTimestamp) { $null } else { [pscustomobject] @{ Subject = $TimestampSubject } }
    }
}

function Assert-PolicyRejects {
    param(
        [Parameter(Mandatory = $true)][object] $Signature,
        [Parameter(Mandatory = $true)][string] $ExpectedMessage
    )

    try {
        Assert-CurtPmeAuthenticodeMetadata -Signature $Signature -FilePath 'test.exe'
    } catch {
        if ($_.Exception.Message -notlike "*$ExpectedMessage*") {
            throw "Expected rejection containing '$ExpectedMessage'; got '$($_.Exception.Message)'."
        }
        return
    }
    throw "Expected signature policy rejection containing '$ExpectedMessage'."
}

Assert-CurtPmeAuthenticodeMetadata -Signature (New-TestSignature) -FilePath 'test.exe'
Assert-CurtPmeAuthenticodeMetadata `
    -Signature (New-TestSignature -Subject 'O=Curt P. Software, CN=Curt P. Software') `
    -FilePath 'test.exe'
Assert-PolicyRejects -Signature (New-TestSignature -Status 'UnknownError') -ExpectedMessage 'must be Valid'
Assert-PolicyRejects -Signature (New-TestSignature -Subject 'CN=CurtPME') -ExpectedMessage 'leaf subject'
Assert-PolicyRejects -Signature (New-TestSignature -MissingSigner) -ExpectedMessage 'signer certificate is missing'
Assert-PolicyRejects -Signature (New-TestSignature -MissingTimestamp) -ExpectedMessage 'timestamp is missing'
Assert-PolicyRejects `
    -Signature (New-TestSignature -TimestampSubject 'CN=Other Timestamp Authority') `
    -ExpectedMessage 'timestamp signer must be DigiCert'

Write-Host 'Windows signature policy regression tests passed.'
