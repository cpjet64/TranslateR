$ErrorActionPreference = "Stop"

git config core.hooksPath .githooks
Write-Host "TranslateR git hooks enabled via core.hooksPath=.githooks"
Write-Host "Pre-push now runs scripts/ci/coverage.ps1 before allowing a push."
