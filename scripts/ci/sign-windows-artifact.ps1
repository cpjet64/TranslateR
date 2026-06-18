param(
    [Parameter(Mandatory = $true)]
    [string] $Path
)

$ErrorActionPreference = "Stop"

function Get-RequiredEnv {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "Required environment variable '$Name' is not set."
    }

    return $value
}

function Get-FileHashHex {
    param(
        [Parameter(Mandatory = $true)]
        [string] $FilePath
    )

    $resolvedPath = [System.IO.Path]::GetFullPath($FilePath)
    if (-not [System.IO.File]::Exists($resolvedPath)) {
        throw "File not found for SHA-256 hashing: $resolvedPath"
    }

    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($resolvedPath)
    try {
        return [System.BitConverter]::ToString($sha.ComputeHash($stream)).Replace("-", "").ToLowerInvariant()
    } finally {
        $stream.Dispose()
        $sha.Dispose()
    }
}

function Assert-AuthenticodeSignature {
    param(
        [Parameter(Mandatory = $true)]
        [string] $FilePath
    )

    if (Get-Command Get-AuthenticodeSignature -ErrorAction SilentlyContinue) {
        $signature = Get-AuthenticodeSignature -FilePath $FilePath
        Write-Host "Authenticode status: $($signature.Status)"
        if ($signature.SignerCertificate) {
            Write-Host "Authenticode signer: $($signature.SignerCertificate.Subject)"
        }

        if ($signature.Status -eq "NotSigned" -or -not $signature.SignerCertificate) {
            throw "Signed artifact does not contain an Authenticode signature: $FilePath"
        }
    } else {
        Write-Host "Get-AuthenticodeSignature is unavailable; verified signed output exists and differs from input."
    }
}

$releaseSkip = [Environment]::GetEnvironmentVariable("RELEASE_SKIP")
if ($releaseSkip -eq "true") {
    Write-Host "Skipping CurtPME signing because this package build will not be uploaded as a release: $Path"
    exit 0
}

$isCi = -not [string]::IsNullOrWhiteSpace($env:CI)
$hasSignerConfig = (-not [string]::IsNullOrWhiteSpace($env:CURTPME_SIGNER_URL)) -and (-not [string]::IsNullOrWhiteSpace($env:CURTPME_SIGNER_TOKEN))
if (-not $isCi -and -not $hasSignerConfig) {
    Write-Host "Skipping CurtPME signing outside CI because signer configuration is not set: $Path"
    exit 0
}

if ($isCi -and $env:CI_COMMIT_REF_PROTECTED -ne "true") {
    throw "Refusing to sign Windows artifact for unprotected ref '$env:CI_COMMIT_REF_NAME'."
}

$resolvedPath = [System.IO.Path]::GetFullPath($Path)
if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
    throw "Artifact to sign was not found: $resolvedPath"
}

$directory = [System.IO.Path]::GetDirectoryName($resolvedPath)
$fileName = [System.IO.Path]::GetFileName($resolvedPath)
$baseName = [System.IO.Path]::GetFileNameWithoutExtension($fileName)
$extension = [System.IO.Path]::GetExtension($fileName)
$signedTempPath = Join-Path $directory (".{0}.{1}.signed{2}" -f $baseName, [guid]::NewGuid().ToString("N"), $extension)

$unsignedHash = Get-FileHashHex -FilePath $resolvedPath

$signerUrl = (Get-RequiredEnv "CURTPME_SIGNER_URL").TrimEnd("/")
$signerToken = Get-RequiredEnv "CURTPME_SIGNER_TOKEN"
$endpoint = "$signerUrl/v1/sign/windows-authenticode"
$headers = @{
    "Authorization" = "Bearer $signerToken"
    "X-CurtPME-Project" = Get-RequiredEnv "CI_PROJECT_PATH"
    "X-CurtPME-Ref" = Get-RequiredEnv "CI_COMMIT_REF_NAME"
    "X-CurtPME-Commit" = Get-RequiredEnv "CI_COMMIT_SHA"
    "X-CurtPME-Pipeline" = Get-RequiredEnv "CI_PIPELINE_ID"
    "X-CurtPME-Job" = Get-RequiredEnv "CI_JOB_ID"
}

Write-Host "Submitting Windows artifact to CurtPME signing service: $fileName"
try {
    Invoke-WebRequest `
        -Method Post `
        -Uri $endpoint `
        -Headers $headers `
        -InFile $resolvedPath `
        -ContentType "application/octet-stream" `
        -OutFile $signedTempPath `
        -UseBasicParsing `
        -ErrorAction Stop | Out-Null
} catch {
    if (Test-Path -LiteralPath $signedTempPath -PathType Leaf) {
        Remove-Item -LiteralPath $signedTempPath -Force
    }
    throw "CurtPME Windows signing request failed for $fileName`: $($_.Exception.Message)"
}

if (-not (Test-Path -LiteralPath $signedTempPath -PathType Leaf)) {
    throw "Signed artifact was not created: $signedTempPath"
}

$signedItem = Get-Item -LiteralPath $signedTempPath
if ($signedItem.Length -le 0) {
    Remove-Item -LiteralPath $signedTempPath -Force
    throw "Signed artifact is empty: $signedTempPath"
}

$signedHash = Get-FileHashHex -FilePath $signedTempPath
Write-Host "SHA256 unsigned: $unsignedHash  $fileName"
Write-Host "SHA256 signed:   $signedHash  $fileName"

if ($unsignedHash -eq $signedHash) {
    Remove-Item -LiteralPath $signedTempPath -Force
    throw "Signed artifact did not differ from unsigned input: $resolvedPath"
}

Assert-AuthenticodeSignature -FilePath $signedTempPath
Move-Item -LiteralPath $signedTempPath -Destination $resolvedPath -Force
Write-Host "CurtPME signed artifact written: $resolvedPath"
