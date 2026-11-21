#!/usr/bin/env bash
# Smoke test: --help + --screenshot to tmp PNG. Prints SMOKE_OK on pass.
set -euo pipefail
cd "$(dirname "$0")/../.."
BIN="./target/release/autocompare-gui"
test -x "$BIN" || { echo "missing $BIN, run build-release.sh first"; exit 1; }
"$BIN" --help 2>/dev/null | grep -q "autocompare" || { echo "SMOKE_FAIL: --help"; exit 1; }
echo "--help OK"
PNG="${TMPDIR:-/tmp}/autocompare-smoke-$$.png"
"$BIN" --screenshot "$PNG" >/dev/null 2>&1
test -f "$PNG" || { echo "SMOKE_FAIL: no png"; exit 1; }
python3 - "$PNG" <<'PY'
import sys
p = sys.argv[1]
d = open(p, 'rb').read()
assert d[:8] == b'\x89PNG\r\n\x1a\n', 'bad magic'
assert len(d) >= 8192, f'too small: {len(d)}'
print(f'png OK ({len(d)} bytes)')
PY
rm -f "$PNG"
echo "SMOKE_OK"
