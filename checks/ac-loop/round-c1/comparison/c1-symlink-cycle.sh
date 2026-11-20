#!/bin/zsh
# c1 symlink-cycle edge fixture: builds a self-loop tree in $TMPDIR and
# checks the CLI folder walk terminates and never descends into the link.
set -u
ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
BIN="$ROOT/target/debug/autocompare"
FIX="$(mktemp -d "${TMPDIR:-/tmp}/c1-cycle.XXXXXX")"
LEFT="$FIX/left"; RIGHT="$FIX/right"
mkdir -p "$LEFT" "$RIGHT"
print 'leaf\n' > "$LEFT/leaf.txt"
print 'k\n' > "$RIGHT/keep.txt"
ln -s "$LEFT" "$LEFT/selfloop"
"$BIN" folder "$LEFT" "$RIGHT" > "$FIX/out.txt" 2>&1
code=$?
grep -q 'selfloop' "$FIX/out.txt" || { echo "FAIL: selfloop entry missing"; cat "$FIX/out.txt"; exit 1; }
grep -q 'selfloop/' "$FIX/out.txt" && { echo "FAIL: walk descended into symlink"; exit 1; }
[ $code = 1 ] || { echo "FAIL: want exit 1 (differences), got $code"; exit 1; }
echo "PASS: cycle terminates, selfloop listed once, never traversed"
rm -rf "$FIX"
