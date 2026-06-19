#!/bin/sh
set -eu

# The Ubuntu CI coverage job cannot exercise the Windows-specific atomic save
# implementation. Keep the Windows PowerShell coverage gate strict for it.
ignore_regex='src[\\/](main\.rs|ui[\\/].*|util[\\/]atomic_save\.rs)'
lcov_path='target/coverage.lcov'

cargo llvm-cov --locked --summary-only --ignore-filename-regex "$ignore_regex" --fail-under-functions 100
cargo llvm-cov --locked --lcov --output-path "$lcov_path" --ignore-filename-regex "$ignore_regex"

misses="$(awk '
  /^SF:/ { source = substr($0, 4); next }
  /^end_of_record$/ { source = ""; next }
  source != "" && /^DA:/ {
    payload = substr($0, 4)
    split(payload, parts, ",")
    if (parts[2] == 0) {
      print source ":" parts[1]
    }
  }
' "$lcov_path")"

if [ -n "$misses" ]; then
  printf '%s\n%s\n' "Coverage gate failed. Uncovered source lines:" "$misses" >&2
  exit 1
fi

printf '%s\n' "Coverage gate passed: 100% function coverage and no uncovered included source lines."
