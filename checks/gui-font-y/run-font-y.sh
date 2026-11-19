#!/bin/zsh
# Font-Y zoom check: scale-factor contract + keybinding core + hierarchy, paint-neutral.
# Usage: run-font-y.sh <round-dir>  (writes <round-dir>/font-y-report.md, exits 0 iff PASS)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run-font-y.sh <round-dir>}"
GUI="$ROOT/target/debug/autocompare-gui"
mkdir -p "$ROUND"
: > "$ROUND/font-y-check.log"

# 1. unit tests (zoom model + keybinding core live in bin unit tests)
cargo test -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" > "$ROUND/tests.out" 2>&1 || exit 4

# 2. headless binary + screenshot (paint must stay working)
cargo build -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" >> "$ROUND/font-y-check.log" 2>&1 || exit 2
"$GUI" --screenshot "$ROUND/after.png" > "$ROUND/screenshot.out" 2>&1 || exit 3

# 3. font-zoom evidence rollup (own-words, numbers only)
python3 - "$ROUND" "$ROOT" >> "$ROUND/font-y-check.log" 2>&1 <<'PY'
import re, subprocess, sys
from pathlib import Path
rounddir, root = Path(sys.argv[1]), Path(sys.argv[2])
theme = (root / "crates/gui/src/theme.rs").read_text()
app = (root / "crates/gui/src/app.rs").read_text()
png = (rounddir / "after.png").read_bytes()
tests = (rounddir / "tests.out").read_text()

def has(hay, s): return s in hay

adds = {
    "theme zoom_scale_factor": has(theme, "pub fn zoom_scale_factor(role: TypoRole, level: i8)"),
    "theme zoomed_scale_factor": has(theme, "pub fn zoomed_scale_factor(self, level: i8)"),
    "theme hierarchy test": has(theme, "zoom_scale_stays_in_factor_band_and_keeps_hierarchy"),
    "app ZoomKey": has(app, "pub enum ZoomKey"),
    "app resolve_zoom_key": has(app, "pub fn resolve_zoom_key"),
    "app apply_zoom_key": has(app, "pub fn apply_zoom_key"),
    "app key test": has(app, "font_zoom_keys_resolve_and_apply"),
    "ui feeds resolver": has(app, "Self::resolve_zoom_key(zoom_in, zoom_out, zoom_reset)"),
}
# Paint path unchanged: rich()/rich_zoomed() still share the 0.0 tracking only.
paint_neutral = theme.count("extra_letter_spacing(TYPO_LETTER_SPACING)") == 2
# Frozen: no removed geometry/color lines in MY files; sibling files out of scope.
diff = subprocess.run(["git", "-C", str(root), "diff", "--",
                        "crates/gui/src/theme.rs", "crates/gui/src/app.rs"],
                      capture_output=True, text=True).stdout
removed_frozen = [l for l in diff.splitlines() if l.startswith("-") and not l.startswith("---")
                  and re.search(r"SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_", l)]
# Banned patterns in non-test code of MY files only.
banned = []
for src in (theme, app):
    nontest = src.split("#[cfg(test)]")[0]
    banned += re.findall(r"unwrap\(|expect\(|panic!|todo!", nontest)

tm = re.search(r"test result: ok\. (\d+) passed; (\d+) failed", tests)
passed, failed = (int(tm.group(1)), int(tm.group(2))) if tm else (0, -1)

magic_ok = png[:8] == bytes.fromhex("89504e470d0a1a0a")
size_ok = len(png) >= 8192
distinct = len(set(png))
distinct_ok = distinct > 64

ok = (all(adds.values()) and paint_neutral and not removed_frozen
      and not banned and failed == 0 and passed >= 273
      and magic_ok and size_ok and distinct_ok)
res = "PASS" if ok else "FAIL"
add_lines = [f"- {k}: {'ok' if v else 'MISSING'}" for k, v in adds.items()]
out = [
    f"# Font-Y report ({rounddir.name})",
    *add_lines,
    f"- paint-neutral rich()/rich_zoomed() 0.0 tracking x2: {'ok' if paint_neutral else 'FAIL'}",
    f"- frozen removals in OWNS: {len(removed_frozen)} {'ok' if not removed_frozen else 'FAIL'}",
    f"- banned in non-test OWNS code: {len(banned)} {'ok' if not banned else 'FAIL ' + str(banned[:3])}",
    f"- cargo test: {passed} passed, {failed} failed (expect >=273/0)",
    f"- PNG: {len(png)} bytes (min 8192: {'ok' if size_ok else 'FAIL'}), "
    f"magic {'ok' if magic_ok else 'FAIL'}, distinct {distinct} ({'ok' if distinct_ok else 'FAIL'})",
    "",
    f"RESULT: {res}",
]
(rounddir / "font-y-report.md").write_text("\n".join(out) + "\n")
print("\n".join(out))
sys.exit(0 if res == "PASS" else 1)
PY
