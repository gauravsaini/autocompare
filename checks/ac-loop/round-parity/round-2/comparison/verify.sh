#!/bin/bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/../../../../../target/debug/autocompare"

# 1. Strict comparison should detect difference (exit code 1)
set +e
"$BIN" text "$DIR/grammar_left.txt" "$DIR/grammar_right.txt" > "$DIR/strict_actual.txt"
STRICT_EXIT=$?
set -e
if [ $STRICT_EXIT -ne 1 ]; then
    echo "Expected exit code 1 for strict comparison, got $STRICT_EXIT"
    exit 1
fi
echo "Strict comparison detected whitespace difference (exit 1) - PASS"

# 2. Whitespace-ignored comparison with -b should treat them as identical (exit code 0)
"$BIN" text -b "$DIR/grammar_left.txt" "$DIR/grammar_right.txt" > "$DIR/ignored_actual.txt"
echo "Whitespace-ignored comparison succeeded (exit 0) - PASS"
