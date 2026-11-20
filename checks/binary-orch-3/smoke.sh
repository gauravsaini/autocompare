#!/bin/sh
# orch-3 gate G4: text + folder smoke against fresh /tmp install.
# Exit convention (diff-style): 0 = identical, 1 = differences found, >=2 = error.
set -u
AC="${AC:-/tmp/orch3-test/autocompare}"
FIX_DIR="$(cd "$(dirname "$0")/smoke" && pwd)"
"$AC" text "$FIX_DIR/left/a.txt" "$FIX_DIR/right/a.txt" --color never >/dev/null 2>&1
rc=$?; [ "$rc" -le 1 ] || exit 1
"$AC" folder "$FIX_DIR/left" "$FIX_DIR/right" --color never >/dev/null 2>&1
rc=$?; [ "$rc" -le 1 ] || exit 1
echo "SMOKE-OK"
