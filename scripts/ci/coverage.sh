#!/bin/sh
set -eu

# The Ubuntu CI coverage job cannot exercise the Windows-specific atomic save
# implementation. Keep the Windows PowerShell gate responsible for that file.
#
# Gate on uncovered included source lines from the LCOV export. LLVM function
# coverage can report duplicate zero-count Rust test-binary instantiations even
# when the source lines are exercised, which makes function thresholds unstable
# across platforms.
ignore_regex='src[\\/](main\.rs|test_support\.rs|ui[\\/].*|update\.rs|util[\\/]atomic_save\.rs)'
lcov_path='target/coverage.lcov'

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

printf '%s\n' "Coverage gate passed: no uncovered included source lines."
