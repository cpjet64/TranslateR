param(
    [Parameter(Mandatory = $true)]
    [string] $ArtifactName
)

$ErrorActionPreference = "Stop"

$binName = "translater.exe"
$stageDir = Join-Path "target\package" $ArtifactName
$projectDir = if ($env:CI_PROJECT_DIR) { $env:CI_PROJECT_DIR } else { (Get-Location).Path }
$archiveDir = Join-Path $projectDir "ci-artifacts"
$archivePath = Join-Path $archiveDir "$ArtifactName.zip"
$binaryPath = Join-Path $projectDir "target\release\$binName"
$signScript = Join-Path $projectDir "scripts\ci\sign-windows-artifact.ps1"
$verifySignatureScript = Join-Path $projectDir "scripts\ci\verify-windows-signature.ps1"
$i18nSource = Join-Path $projectDir "release-i18n"
if (-not (Test-Path -LiteralPath $i18nSource)) {
    $i18nSource = Join-Path $projectDir "i18n"
}

cargo build --release
powershell -NoProfile -ExecutionPolicy Bypass -File $signScript -Path $binaryPath
if ($LASTEXITCODE -ne 0) {
    throw "CurtPME Windows signing failed with exit code $LASTEXITCODE."
}
if ($env:RELEASE_SKIP -ne "true") {
    powershell -NoProfile -ExecutionPolicy Bypass -File $verifySignatureScript -Path $binaryPath
    if ($LASTEXITCODE -ne 0) {
        throw "Windows signature verification failed with exit code $LASTEXITCODE."
    }
}

if (Test-Path -LiteralPath $stageDir) {
    Remove-Item -LiteralPath $stageDir -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "LICENSES") | Out-Null
New-Item -ItemType Directory -Force -Path $archiveDir | Out-Null

Copy-Item -LiteralPath $binaryPath -Destination $stageDir
Copy-Item -LiteralPath "README.md" -Destination $stageDir
Copy-Item -LiteralPath "LICENSE" -Destination $stageDir
Copy-Item -LiteralPath "NOTICE.md" -Destination $stageDir
Copy-Item -Path "LICENSES\*" -Destination (Join-Path $stageDir "LICENSES")
if (Test-Path -LiteralPath $i18nSource) {
    Copy-Item -Recurse -LiteralPath $i18nSource -Destination (Join-Path $stageDir "i18n")
}
if (Test-Path -LiteralPath "release-notes.md") {
    Copy-Item -LiteralPath "release-notes.md" -Destination (Join-Path $stageDir "CHANGELOG.md")
} else {
    Copy-Item -LiteralPath "CHANGELOG.md" -Destination $stageDir
}

if (Test-Path -LiteralPath $archivePath) {
    Remove-Item -LiteralPath $archivePath -Force
}

Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $archivePath
if (-not (Test-Path -LiteralPath $archivePath)) {
    throw "Windows package archive was not created: $archivePath"
}
Get-ChildItem -LiteralPath $archiveDir | Select-Object Name, Length | Format-Table -AutoSize
Write-Output $archivePath
