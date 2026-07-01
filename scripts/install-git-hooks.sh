#!/bin/sh
set -eu

git config core.hooksPath .githooks
if command -v chmod >/dev/null 2>&1; then
  chmod +x .githooks/pre-commit .githooks/pre-push scripts/ci/coverage.sh scripts/ci/prepush.sh scripts/install-git-hooks.sh
fi

printf '%s\n' "TranslateR git hooks enabled via core.hooksPath=.githooks"
printf '%s\n' "Pre-commit now runs cargo fmt before allowing a commit."
printf '%s\n' "Pre-push now runs scripts/ci/prepush.sh before allowing a push."
