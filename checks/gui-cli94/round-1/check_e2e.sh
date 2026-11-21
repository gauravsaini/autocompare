#!/bin/sh
# End-to-end binary check: exit codes 0/1/2, -ro guard, --report write.
set -u
BIN="${BIN:-./target/debug/autocompare}"
cargo build -p autocompare-cli --bin autocompare >/dev/null 2>&1
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
printf 'alpha\nbeta\n' > "$TMP/a.txt"
printf 'alpha\nbeta\n' > "$TMP/b.txt"
printf 'alpha\nBETA\n' > "$TMP/c.txt"
printf 'base\n' > "$TMP/base.txt"
printf 'left\n' > "$TMP/left.txt"
printf 'right\n' > "$TMP/right.txt"

"$BIN" text "$TMP/a.txt" "$TMP/b.txt" >/dev/null 2>&1
[ "$?" -eq 0 ] || { echo "same expected 0"; exit 1; }
"$BIN" text "$TMP/a.txt" "$TMP/c.txt" >/dev/null 2>&1
[ "$?" -eq 1 ] || { echo "diff expected 1"; exit 1; }
"$BIN" --completely-invalid-flag >/dev/null 2>&1
[ "$?" -eq 2 ] || { echo "bad flag expected 2"; exit 1; }
"$BIN" -ro merge "$TMP/base.txt" "$TMP/left.txt" "$TMP/right.txt" --output "$TMP/out.txt" >/dev/null 2>&1
[ "$?" -eq 2 ] || { echo "-ro merge expected 2"; exit 1; }
"$BIN" text "$TMP/a.txt" "$TMP/c.txt" --report "html:$TMP/rep.html" >/dev/null 2>&1
[ "$?" -eq 1 ] || { echo "report run expected 1"; exit 1; }
[ -s "$TMP/rep.html" ] || { echo "report file missing"; exit 1; }
"$BIN" -automerge merge "$TMP/base.txt" "$TMP/left.txt" "$TMP/right.txt" --mergeoutput "$TMP/m.txt" >/dev/null 2>&1 || true
[ -s "$TMP/m.txt" ] || { echo "mergeoutput missing"; exit 1; }
echo "E2E_OK"
