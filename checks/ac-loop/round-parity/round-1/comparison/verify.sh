#!/bin/bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/../../../../../target/debug/autocompare"

# 1. Clean 3-way merge test
"$BIN" merge "$DIR/clean_base.txt" "$DIR/clean_left.txt" "$DIR/clean_right.txt" > "$DIR/clean_actual.txt"
diff -u "$DIR/clean_expected.txt" "$DIR/clean_actual.txt"
echo "Clean 3-way merge passed"

# 2. Conflict 3-way merge test
set +e
"$BIN" merge "$DIR/merge3_base.txt" "$DIR/merge3_left.txt" "$DIR/merge3_right.txt" > "$DIR/conflict_actual.txt" 2>/dev/null
EXIT_CODE=$?
set -e
if [ $EXIT_CODE -ne 1 ]; then
    echo "Expected exit code 1 for conflicts, got $EXIT_CODE"
    exit 1
fi
diff -u "$DIR/expected_conflict.txt" "$DIR/conflict_actual.txt"
echo "Conflicting 3-way merge with conflict markers passed"
