$ErrorActionPreference = "Stop"

git config core.hooksPath .githooks
Write-Host "TranslateR git hooks enabled via core.hooksPath=.githooks"
Write-Host "Pre-commit now runs cargo fmt before allowing a commit."
Write-Host "Pre-push now runs scripts/ci/prepush.ps1 before allowing a push."
