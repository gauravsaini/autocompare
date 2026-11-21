#!/usr/bin/env bash
# Banned-pattern check for release scripts. Prints BANNED_ZERO on pass.
set -euo pipefail
cd "$(dirname "$0")"
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck ./*.sh || exit 1
  echo "shellcheck OK"
else
  echo "shellcheck not available, skipped"
fi
if grep -rnE 'curl|wget|sudo|rm -rf /' ./*.sh ./INSTALL.md 2>/dev/null | grep -v check-banned; then
  echo "BANNED_FOUND"
  exit 1
fi
echo "BANNED_ZERO"
