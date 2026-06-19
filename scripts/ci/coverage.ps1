$ErrorActionPreference = "Stop"

$ignoreRegex = 'src[\\/](main\.rs|ui[\\/].*|update\.rs)'
$lcovPath = "target/coverage.lcov"

cargo llvm-cov --locked --summary-only --ignore-filename-regex $ignoreRegex --fail-under-functions 100
cargo llvm-cov --locked --lcov --output-path $lcovPath --ignore-filename-regex $ignoreRegex

$misses = New-Object System.Collections.Generic.List[string]
$sourceFile = $null

foreach ($line in Get-Content -LiteralPath $lcovPath) {
    if ($line.StartsWith("SF:")) {
        $sourceFile = $line.Substring(3)
        continue
    }

    if ($line -eq "end_of_record") {
        $sourceFile = $null
        continue
    }

    if ($null -eq $sourceFile -or -not $line.StartsWith("DA:")) {
        continue
    }

    $parts = $line.Substring(3).Split(",")
    if ($parts.Count -lt 2) {
        continue
    }

    if ([int]$parts[1] -eq 0) {
        $misses.Add("${sourceFile}:$($parts[0])")
    }
}

if ($misses.Count -gt 0) {
    Write-Error "Coverage gate failed. Uncovered source lines:`n$($misses -join "`n")"
    exit 1
}

Write-Host "Coverage gate passed: 100% function coverage and no uncovered included source lines."
