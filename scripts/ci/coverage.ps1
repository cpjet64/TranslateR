$ErrorActionPreference = "Stop"

$ignoreRegex = 'src[\\/](main\.rs|test_support\.rs|ui[\\/].*|update\.rs)'
$lcovPath = "target/coverage.lcov"

# Gate on uncovered included source lines from the LCOV export. LLVM function
# coverage can report duplicate zero-count Rust test-binary instantiations even
# when the source lines are exercised, which makes function thresholds unstable
# across platforms.
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

Write-Host "Coverage gate passed: no uncovered included source lines."
