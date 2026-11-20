#!/bin/bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/../../../../../target/debug/autocompare"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

cp -r "$DIR/sync_left" "$TMP_DIR/left"
cp -r "$DIR/sync_right" "$TMP_DIR/right"

# 1. Dry run test
"$BIN" sync --direction l2r --prune "$TMP_DIR/left" "$TMP_DIR/right" > "$TMP_DIR/dry_run.txt" || true
grep -q "→ nested/child.txt" "$TMP_DIR/dry_run.txt"
grep -q "× stale.txt" "$TMP_DIR/dry_run.txt"
grep -q "2 step(s) planned" "$TMP_DIR/dry_run.txt"
echo "Sync dry run plan verified - PASS"

# 2. Apply test with prune
"$BIN" sync --direction l2r --prune --apply "$TMP_DIR/left" "$TMP_DIR/right" > "$TMP_DIR/apply.txt"
grep -q "copied: 1" "$TMP_DIR/apply.txt"
grep -q "deleted: 1" "$TMP_DIR/apply.txt"

# 3. Target verification
[ -f "$TMP_DIR/right/nested/child.txt" ] || { echo "child.txt was not copied"; exit 1; }
[ ! -f "$TMP_DIR/right/stale.txt" ] || { echo "stale.txt was not pruned"; exit 1; }
echo "Sync apply and pruning verified - PASS"
