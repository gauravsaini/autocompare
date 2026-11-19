#!/bin/zsh
# Geo+Typo one-round check: exquisite UI loop (geometry first, typography second, colors out of scope).
# Usage: run-geo-typo.sh <round-dir>  (writes <round-dir>/geo-typo-report.md, exits 0 iff PASS)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run-geo-typo.sh <round-dir>}"
GUI="$ROOT/target/debug/autocompare-gui"
mkdir -p "$ROUND"
cargo build -p autocompare-gui --quiet --manifest-path "$ROOT/Cargo.toml" || exit 2
"$GUI" --screenshot "$ROUND/after.png" > "$ROUND/screenshot.out" 2>&1 || exit 3
cargo test -p autocompare-gui --quiet --manifest-path "$ROOT/Cargo.toml" > "$ROUND/tests.out" 2>&1 || exit 4
python3 - "$ROUND/after.png" "$ROOT/crates/gui/src" > "$ROUND/geo-typo-report.md" <<'PY'
import sys, re
from pathlib import Path
png = Path(sys.argv[1]); srcdir = Path(sys.argv[2])
b = png.read_bytes()
magic_ok = b[:8] == bytes.fromhex('89504e470d0a1a0a')
size_ok = len(b) >= 8192
distinct = len(set(b)); blank_ok = distinct > 64
src = "".join((p.read_text() for p in srcdir.glob("*.rs")))
spaces = re.findall(r"add_space\(([\d.]+)\)", src)
widths = re.findall(r"(?:set_width|desired_width|with\S*width)\s*\(?([^\)]*)\)?", src)
mono = src.count("monospace()"); strong = src.count(".strong()"); small = src.count(".small()")
row_h = src.count("text_style_height"); grid = src.count("Grid::new"); rich_size = re.findall(r"\.size\(([\d.]+)\)", src)
color_touches = src.count("Color32::from_rgb") + src.count("Visuals::")
print(f"# Geo-Typo report ({png.parent.name})")
print(f"- PNG: {len(b)} bytes (min 8192: {'ok' if size_ok else 'FAIL'}), magic {'ok' if magic_ok else 'FAIL'}, distinct {distinct} ({'ok' if blank_ok else 'FAIL'})")
print(f"- geometry: add_space values {sorted(set(spaces))}, Grid::{grid}, text_style_height x{row_h}")
print(f"- widths sample: {len(widths)} width calls")
print(f"- typography: monospace x{mono}, strong x{strong}, small x{small}, .size values {sorted(set(rich_size))}, row_height x{row_h}")
print(f"- colors untouched check: Color32/Visuals touches {color_touches} (info only, colors out of scope)")
res = 'RESULT: PASS' if (magic_ok and size_ok and blank_ok) else 'RESULT: FAIL'
print(res)
PY
grep -q 'RESULT: PASS' "$ROUND/geo-typo-report.md"
