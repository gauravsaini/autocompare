#!/bin/zsh
# Visual one-round check: GUI --screenshot smoke for fixtures the app renders.
# Usage: run-visual.sh <round-dir>  (writes <round-dir>/visual-report.md, exits 0 iff PASS)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run-visual.sh <round-dir>}"
GUI="$ROOT/target/debug/autocompare-gui"
mkdir -p "$ROUND"
cargo build -p autocompare-gui --quiet --manifest-path "$ROOT/Cargo.toml" || exit 2
"$GUI" --screenshot "$ROUND/after.png" > "$ROUND/screenshot.out" 2>&1 || exit 3
python3 - "$ROUND/after.png" > "$ROUND/visual-report.md" <<'PY'
import sys
from pathlib import Path
p = Path(sys.argv[1])
b = p.read_bytes()
magic_ok = b[:8] == bytes.fromhex('89504e470d0a1a0a')
size_ok = len(b) >= 8192
distinct = len(set(b))
blank_ok = distinct > 64
res = 'RESULT: PASS' if (magic_ok and size_ok and blank_ok) else 'RESULT: FAIL'
print(f'# Visual parity ({p.parent.name})')
print(f'- size: {len(b)} bytes (min 8192: {"ok" if size_ok else "FAIL"})')
print(f'- PNG magic: {"ok" if magic_ok else "FAIL"}')
print(f'- distinct bytes: {distinct} (non-blank: {"ok" if blank_ok else "FAIL"})')
print(f'{res}')
PY
grep -q 'RESULT: PASS' "$ROUND/visual-report.md"
