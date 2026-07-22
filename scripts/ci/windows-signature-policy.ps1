$script:CurtPmeExpectedAuthenticodeSubjects = @(
    'CN=Curt P. Software, O=Curt P. Software',
    'O=Curt P. Software, CN=Curt P. Software'
)

function Assert-CurtPmeAuthenticodeMetadata {
    param(
        [Parameter(Mandatory = $true)]
        [object] $Signature,

        [Parameter(Mandatory = $true)]
        [string] $FilePath
    )

    if ([string] $Signature.Status -cne 'Valid') {
        throw "Authenticode status must be Valid for '$FilePath'; got '$($Signature.Status)'."
    }
    if ($null -eq $Signature.SignerCertificate) {
        throw "Authenticode signer certificate is missing: $FilePath"
    }

    $subject = [string] $Signature.SignerCertificate.Subject
    if ($script:CurtPmeExpectedAuthenticodeSubjects -cnotcontains $subject) {
        throw "Authenticode leaf subject must be 'CN=Curt P. Software, O=Curt P. Software' for '$FilePath'; got '$subject'."
    }
    if ($null -eq $Signature.TimeStamperCertificate) {
        throw "Authenticode RFC3161 timestamp is missing: $FilePath"
    }
    $timestampSubject = [string] $Signature.TimeStamperCertificate.Subject
    if ($timestampSubject -notmatch 'DigiCert') {
        throw "Authenticode timestamp signer must be DigiCert for '$FilePath'; got '$timestampSubject'."
    }
}

function Assert-CurtPmeAuthenticodeSignature {
    param(
        [Parameter(Mandatory = $true)]
        [string] $FilePath
    )

    $authenticode = Get-Command Get-AuthenticodeSignature -ErrorAction SilentlyContinue
    if (-not $authenticode) {
        throw 'Get-AuthenticodeSignature is required for Windows release verification.'
    }

    $resolvedPath = [System.IO.Path]::GetFullPath($FilePath)
    $signature = Get-AuthenticodeSignature -LiteralPath $resolvedPath
    Assert-CurtPmeAuthenticodeMetadata -Signature $signature -FilePath $resolvedPath

    Write-Host "Authenticode status: $($signature.Status)"
    Write-Host "Authenticode signer: $($signature.SignerCertificate.Subject)"
    Write-Host "Authenticode timestamper: $($signature.TimeStamperCertificate.Subject)"

    $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if (-not $signtool) {
        Write-Host 'signtool.exe is unavailable; Authenticode policy verification passed with Windows PowerShell.'
        return
    }

    $verifyOutput = @(& $signtool.Source verify /pa /all /v /tw $resolvedPath 2>&1)
    $verifyExitCode = $LASTEXITCODE
    $verifyOutput | ForEach-Object { Write-Host $_ }
    if ($verifyExitCode -ne 0) {
        throw "signtool verification failed for '$resolvedPath' with exit code $verifyExitCode."
    }
}
