#!/bin/zsh
# Exquisite harness: geometry-first, typography-second, colors frozen.
# Usage: tools/exquisite/run.sh <round-dir> [gui-src-dir]
# Writes <round-dir>/exquisite-report.md, exits 0 iff PASS.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run.sh <round-dir>}"
SRCDIR="${2:-$ROOT/crates/gui/src}"
GUI="$ROOT/target/debug/autocompare-gui"
mkdir -p "$ROUND"
cargo build -p autocompare-gui --quiet --manifest-path "$ROOT/Cargo.toml" || exit 2
"$GUI" --screenshot "$ROUND/after.png" > "$ROUND/screenshot.out" 2>&1 || exit 3
cargo test -p autocompare-gui --quiet --manifest-path "$ROOT/Cargo.toml" > "$ROUND/tests.out" 2>&1 || exit 4
python3 - "$ROUND/after.png" "$SRCDIR" > "$ROUND/exquisite-report.md" <<'PY'
import sys, re
from pathlib import Path
png = Path(sys.argv[1]); srcdir = Path(sys.argv[2])
b = png.read_bytes()
magic_ok = b[:8] == bytes.fromhex('89504e470d0a1a0a')
size_ok = len(b) >= 8192
distinct = len(set(b)); blank_ok = distinct > 64
src = "".join(p.read_text() for p in srcdir.glob("*.rs"))
def count(pat): return len(re.findall(pat, src))
spaces = sorted(set(re.findall(r"SPACE_(XS|SM|MD|LG|XL)", src)))
margins = count(r"OUTER_MARGIN|SECTION_GAP|TIGHT_GAP|FOOTER_GAP|TAB_BOTTOM_GAP")
grids = count(r"Grid::new")
rowh = count(r"text_style_height|line_height_px|line_height\(\)")
widths = count(r"set_width|desired_width|with_.*width|available_width")
gutter = count(r"gutter_|GUTTER_")
mono = src.count("monospace()"); strong = src.count(".strong()")
sizes = sorted(set(re.findall(r"\.size\(([\d.]+)\)", src)))
typo_roles = count(r"TypoRole::")
weights = sorted(set(re.findall(r"TYPO_WEIGHT_\w+", src)))
lh = sorted(set(re.findall(r"TYPO_LINE_HEIGHT_\w+", src)))
ls = sorted(set(re.findall(r"TYPO_LETTER_SPACING|extra_letter_spacing", src)))
colors = src.count("Color32::from_rgb") + src.count("Visuals::")
print(f"# Exquisite report ({png.parent.name})")
print(f"- PNG: {len(b)} bytes (min 8192: {'ok' if size_ok else 'FAIL'}), magic {'ok' if magic_ok else 'FAIL'}, distinct {distinct} ({'ok' if blank_ok else 'FAIL'})")
print(f"- GEOMETRY: SPACE tokens {spaces}, margin/gap consts x{margins}, Grid x{grids}, row-h x{rowh}, width calls x{widths}, gutter x{gutter}")
print(f"- TYPO: TypoRole x{typo_roles}, monospace x{mono}, strong x{strong}, .size {sizes}, weights {weights}, line-heights {lh}, spacing {ls}")
print(f"- COLORS frozen check: Color32/Visuals touches {colors} (info only, must not grow for geo-typo waves)")
res = 'RESULT: PASS' if (magic_ok and size_ok and blank_ok) else 'RESULT: FAIL'
print(res)
PY
grep -q 'RESULT: PASS' "$ROUND/exquisite-report.md"
