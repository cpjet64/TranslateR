$ErrorActionPreference = "Stop"

Write-Host "TranslateR local CI: cargo fmt"
cargo fmt --all -- --check

Write-Host "TranslateR local CI: cargo clippy"
cargo clippy --locked --all-targets --all-features -- -D warnings

Write-Host "TranslateR local CI: coverage gate"
& "$PSScriptRoot\coverage.ps1"
