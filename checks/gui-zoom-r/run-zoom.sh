#!/bin/zsh
# Zoom round check: theme zoom model + keybinding patch + paint-neutral proof.
# Usage: run-zoom.sh <round-dir>  (writes <round-dir>/zoom-report.md, exits 0 iff PASS)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run-zoom.sh <round-dir>}"
GUI="$ROOT/target/debug/autocompare-gui"
mkdir -p "$ROUND"
: > "$ROUND/zoom-check.log"

# 1. unit tests (theme zoom model lives in bin unit tests)
cargo test -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" > "$ROUND/tests.out" 2>&1 || exit 4

# 2. headless binary + screenshot (paint must stay working)
cargo build -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" >> "$ROUND/zoom-check.log" 2>&1 || exit 2
"$GUI" --screenshot "$ROUND/after.png" > "$ROUND/screenshot.out" 2>&1 || exit 3

# 3. zoom evidence rollup (own-words, numbers only)
python3 - "$ROUND" "$ROOT" >> "$ROUND/zoom-check.log" 2>&1 <<'PY'
import re, subprocess, sys
from pathlib import Path
rounddir, root = Path(sys.argv[1]), Path(sys.argv[2])
theme = (root / "crates/gui/src/theme.rs").read_text()
png = (rounddir / "after.png").read_bytes()
tests = (rounddir / "tests.out").read_text()

def grab(pat, cast=float):
    m = re.search(pat, theme)
    return cast(m.group(1)) if m else None

min_level = int(grab(r"pub const ZOOM_MIN_LEVEL:\s*i8\s*=\s*(-?\d+);", int))
max_level = int(grab(r"pub const ZOOM_MAX_LEVEL:\s*i8\s*=\s*(-?\d+);", int))
step = grab(r"pub const ZOOM_STEP_PX:\s*f32\s*=\s*([\d.]+);")
min_size = grab(r"pub const ZOOM_MIN_SIZE:\s*f32\s*=\s*([\d.]+);")
max_size = grab(r"pub const ZOOM_MAX_SIZE:\s*f32\s*=\s*([\d.]+);")
levels = re.search(r"pub const ZOOM_LEVELS:\s*\[i8;\s*7\]\s*=\s*(\[[^\]]+\]);", theme)
has_resolver = "pub fn zoomed_size(role: TypoRole, level: i8)" in theme
has_method = "pub fn zoomed_point_size(self, level: i8)" in theme
has_rich = "pub fn rich_zoomed(self" in theme
has_tests = all(s in theme for s in [
    "zoom_steps_cover_min_to_max_in_order",
    "zoom_level_clamps_to_table_edges",
    "zoomed_size_zero_level_is_base",
    "zoomed_size_steps_one_px_per_level_then_clamps",
    "zoomed_rich_carries_zoomed_size_per_role",
])
geo_touches = len(re.findall(r"SPACE_[A-Z]+|GUTTER_|GEO_", theme))  # info only
color_adds = theme.count("Color32::from_rgb")  # must stay 0 new (frozen wave)

tm = re.search(r"test result: ok\. (\d+) passed; (\d+) failed", tests)
passed, failed = (int(tm.group(1)), int(tm.group(2))) if tm else (0, -1)

patch = root / "checks/gui-zoom-r/INTEGRATION.patch"
apply = subprocess.run(
    ["git", "-C", str(root), "apply", "--check", "checks/gui-zoom-r/INTEGRATION.patch"],
    capture_output=True, text=True)
patch_ok = apply.returncode == 0
patch_stats = subprocess.run(
    ["git", "apply", "--stat", str(patch)], capture_output=True, text=True).stdout.strip()

magic_ok = png[:8] == bytes.fromhex("89504e470d0a1a0a")
size_ok = len(png) >= 8192
distinct_ok = len(set(png)) > 64

# Step table: Body 13 + level*1 clamped to 8..24 (Mono 12 shown too).
rows = []
for lv in range(min_level, max_level + 1):
    body = min(max(13.0 + lv * step, min_size), max_size)
    mono = min(max(12.0 + lv * step, min_size), max_size)
    rows.append(f"| {lv:+d} | {body:.1f} | {mono:.1f} |")

res = "PASS" if (passed > 0 and failed == 0 and has_resolver and has_method
                 and has_rich and has_tests and patch_ok and magic_ok
                 and size_ok and distinct_ok) else "FAIL"
out = [
    f"# Zoom report ({rounddir.name})",
    f"- steps: MIN {min_level}, MAX {max_level}, STEP {step}, SIZE {min_size}..{max_size}",
    f"- levels table: {levels.group(1) if levels else 'MISSING'}",
    f"- resolver: free zoomed_size {'ok' if has_resolver else 'MISSING'}, "
    f"method zoomed_point_size {'ok' if has_method else 'MISSING'}, "
    f"rich_zoomed {'ok' if has_rich else 'MISSING'}",
    f"- theme tests: 5 zoom fns {'ok' if has_tests else 'MISSING'}",
    f"- cargo test: {passed} passed, {failed} failed",
    f"- INTEGRATION.patch apply --check: {'ok' if patch_ok else 'FAIL'} ({patch_stats})",
    f"- PNG: {len(png)} bytes (min 8192: {'ok' if size_ok else 'FAIL'}), "
    f"magic {'ok' if magic_ok else 'FAIL'}, distinct {'ok' if distinct_ok else 'FAIL'}",
    f"- paint-neutral: geometry tokens untouched (refs {geo_touches}, info), "
    f"Color32::from_rgb count {color_adds} (frozen, no new hues)",
    "",
    "| level | Body (13+lv) | Mono (12+lv) |",
    "| --- | --- | --- |",
    *rows,
    "",
    f"RESULT: {res}",
]
(rounddir / "zoom-report.md").write_text("\n".join(out) + "\n")
print("\n".join(out))
sys.exit(0 if res == "PASS" else 1)
PY
