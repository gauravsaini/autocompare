#!/bin/sh
# Banned scan: diff sirf owns paths (xtask/**, checks/gui-sec99/**) + koi asli secret shape nahi.
set -eu
cd "$(dirname "$0")/../../.."
# 1. changed paths (worktree + index vs HEAD)
files=$(git status --porcelain=v1 | awk '{print $2}')
bad=""
for f in $files; do
  case "$f" in
    xtask/*|checks/gui-sec99/*) ;;
    "") ;;
    *) bad="$bad $f" ;;
  esac
done
# stray check explicitly
if git status --porcelain=v1 | grep -q "checks/gui-sessionpaint-by"; then
  echo "STRAY gui-sessionpaint-by TOUCHED"
  exit 1
fi
if [ -n "$bad" ]; then
  echo "OUT_OF_SCOPE:$bad"
  exit 1
fi
# 2. prod diff me asli secret shape nahi (lamba token ya private-key banner).
# NOTE: banner ko parts me jodte hain taaki ye script khud scanner me FP na de.
banner="BEGIN ""PRIVATE ""KEY"
if git diff HEAD -- xtask checks/gui-sec99 | grep -Eq "ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|$banner"; then
  echo "BANNED_SECRET_IN_DIFF"
  exit 1
fi
echo "BANNED_SCAN_OK"
