#!/bin/sh
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="${1:-/tmp/target-agyx-binmix/release/autocompare}"
FX="$ROOT/tools/parity-harness/fixtures"
OUT="$ROOT/checks/binary-mix"

if [ ! -x "$BIN" ]; then
  echo "Binary not found or not executable: $BIN"
  exit 1
fi

"$BIN" --help > "$OUT/cli-help.out" 2>&1 || exit 1
"$BIN" --version > "$OUT/cli-version.out" 2>&1 || exit 1

# Exit codes: 0 = identical, 1 = differences, >=2 = error
set +e
"$BIN" text "$FX/t_left.txt" "$FX/t_right.txt" > "$OUT/cli-text-diff.out" 2>&1
diff_exit=$?

"$BIN" text "$FX/t_same.txt" "$FX/t_same.txt" > "$OUT/cli-text-same.out" 2>&1
same_exit=$?
set -e

if [ "$diff_exit" -eq 1 ] && [ "$same_exit" -eq 0 ]; then
  echo "CLI_SMOKE_OK"
  exit 0
else
  echo "CLI smoke failed: diff_exit=$diff_exit (expected 1), same_exit=$same_exit (expected 0)"
  exit 2
fi
