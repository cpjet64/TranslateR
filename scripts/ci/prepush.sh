#!/bin/sh
set -eu

printf '%s\n' "TranslateR local CI: cargo fmt"
cargo fmt --all -- --check

printf '%s\n' "TranslateR local CI: cargo clippy"
cargo clippy --locked --all-targets --all-features -- -D warnings

printf '%s\n' "TranslateR local CI: coverage gate"
sh scripts/ci/coverage.sh
