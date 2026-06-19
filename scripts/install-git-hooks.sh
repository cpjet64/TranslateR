#!/bin/sh
set -eu

git config core.hooksPath .githooks
if command -v chmod >/dev/null 2>&1; then
  chmod +x .githooks/pre-push scripts/ci/coverage.sh scripts/install-git-hooks.sh
fi

printf '%s\n' "TranslateR git hooks enabled via core.hooksPath=.githooks"
printf '%s\n' "Pre-push now runs scripts/ci/coverage.sh before allowing a push."
