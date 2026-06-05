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

cargo build --release

if (Test-Path -LiteralPath $stageDir) {
    Remove-Item -LiteralPath $stageDir -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "LICENSES") | Out-Null
New-Item -ItemType Directory -Force -Path $archiveDir | Out-Null

Copy-Item -LiteralPath "target\release\$binName" -Destination $stageDir
Copy-Item -LiteralPath "README.md" -Destination $stageDir
Copy-Item -LiteralPath "LICENSE" -Destination $stageDir
Copy-Item -LiteralPath "NOTICE.md" -Destination $stageDir
Copy-Item -LiteralPath "LICENSES\*" -Destination (Join-Path $stageDir "LICENSES")

if (Test-Path -LiteralPath $archivePath) {
    Remove-Item -LiteralPath $archivePath -Force
}

Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $archivePath
if (-not (Test-Path -LiteralPath $archivePath)) {
    throw "Windows package archive was not created: $archivePath"
}
Get-ChildItem -LiteralPath $archiveDir | Select-Object Name, Length | Format-Table -AutoSize
Write-Output $archivePath
