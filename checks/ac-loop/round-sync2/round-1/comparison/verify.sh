#!/bin/bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/../../../../../target/debug/autocompare"
LEFT="$DIR/r1-left"
RIGHT="$DIR/r1-right"
# Equalize mtimes so keep.txt compares Same on any checkout.
touch -r "$LEFT/keep.txt" "$RIGHT/keep.txt"

# UpdateRight dry-run: 1 copy, 1 skip (right-extra left alone)
OUT1="$("$BIN" sync --color never "$LEFT" "$RIGHT" 2>&1 || true)"
echo "$OUT1" | grep -q "1 step(s) planned, 1 skipped" || { echo "update-right preview mismatch"; echo "$OUT1"; exit 1; }
echo "$OUT1" | grep -q "only-left.txt" || { echo "missing only-left step"; exit 1; }

# Mirror dry-run: copy + prune delete = 2 steps, 0 skipped
OUT2="$("$BIN" sync --color never --prune "$LEFT" "$RIGHT" 2>&1 || true)"
echo "$OUT2" | grep -q "2 step(s) planned, 0 skipped" || { echo "mirror preview mismatch"; echo "$OUT2"; exit 1; }
echo "$OUT2" | grep -q "right-extra.txt" || { echo "missing right-extra delete step"; exit 1; }

# Consent gate is engine-level (apply_with_consent): dry runs above wrote nothing
test -f "$LEFT/only-left.txt"
test -f "$RIGHT/right-extra.txt"
test ! -f "$RIGHT/only-left.txt"

echo "round-1 consent/preview fixtures passed"
