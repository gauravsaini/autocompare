#!/bin/zsh
# Typo-U exquisite check: tokens + zoom-clamp proof, paint-neutral.
# Usage: run-typo-u.sh <round-dir>  (writes <round-dir>/typo-u-report.md, exits 0 iff PASS)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run-typo-u.sh <round-dir>}"
GUI="$ROOT/target/debug/autocompare-gui"
mkdir -p "$ROUND"
: > "$ROUND/typo-u-check.log"

# 1. unit tests (exquisite tokens + zoom clamps live in bin unit tests)
cargo test -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" > "$ROUND/tests.out" 2>&1 || exit 4

# 2. headless binary + screenshot (paint must stay working, bit-identical intent)
cargo build -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" >> "$ROUND/typo-u-check.log" 2>&1 || exit 2
"$GUI" --screenshot "$ROUND/after.png" > "$ROUND/screenshot.out" 2>&1 || exit 3

# 3. exquisite evidence rollup (own-words, numbers only)
python3 - "$ROUND" "$ROOT" >> "$ROUND/typo-u-check.log" 2>&1 <<'PY'
import re, subprocess, sys
from pathlib import Path
rounddir, root = Path(sys.argv[1]), Path(sys.argv[2])
theme = (root / "crates/gui/src/theme.rs").read_text()
png = (rounddir / "after.png").read_bytes()
tests = (rounddir / "tests.out").read_text()

def has(s): return s in theme

tokens = {
    "TYPO_UI_SANS_STACK": has("pub const TYPO_UI_SANS_STACK"),
    "TYPO_MONO_STACK": has("pub const TYPO_MONO_STACK"),
    "W400/500/600": all(s in theme for s in ["TYPO_WEIGHT_REGULAR", "TYPO_WEIGHT_MEDIUM", "TYPO_WEIGHT_SEMIBOLD"]),
    "LH 1.2/1.45/1.35": all(s in theme for s in ["TYPO_LINE_HEIGHT_CHROME", "TYPO_LINE_HEIGHT_BODY", "TYPO_LINE_HEIGHT_MONO"]),
    "TRACKING_EM": has("pub const TYPO_TRACKING_HEADING_EM"),
    "font_stack": has("pub const fn font_stack"),
    "weight_value": has("pub const fn weight_value"),
    "line_height": has("pub const fn line_height("),
    "line_height_px": has("pub const fn line_height_px"),
    "letter_spacing_em": has("pub const fn letter_spacing_em"),
    "is_tabular": has("pub const fn is_tabular"),
    "exq_test": has("typo_exquisite_tokens_carry_hierarchy"),
    "zoom_clamp_test": has("typo_exquisite_zoom_clamps_every_role"),
}
# Paint path unchanged: rich() still uses shared 0.0 tracking only.
paint_neutral = theme.count("extra_letter_spacing(TYPO_LETTER_SPACING)") == 2
# Frozen: no removed geometry/color lines; only additions allowed.
diff = subprocess.run(["git", "-C", str(root), "diff", "--", "crates/gui/src/theme.rs"],
                      capture_output=True, text=True).stdout
removed_frozen = [l for l in diff.splitlines() if l.startswith("-") and not l.startswith("---")
                  and re.search(r"SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_", l)]
# Banned patterns in non-test code: strip test module tail.
nontest = theme.split("#[cfg(test)]")[0]
banned = re.findall(r"unwrap|expect\(|panic!|todo!", nontest)

tm = re.search(r"test result: ok\. (\d+) passed; (\d+) failed", tests)
passed, failed = (int(tm.group(1)), int(tm.group(2))) if tm else (0, -1)

magic_ok = png[:8] == bytes.fromhex("89504e470d0a1a0a")
distinct = len(set(png))
distinct_ok = distinct > 64

# Bit-identical vs Wave-R baseline PNG (paint-neutral proof).
base = root / "checks/gui-zoom-r/round-R-1/after.png"
identical = base.exists() and base.read_bytes() == png

ok = (all(tokens.values()) and paint_neutral and not removed_frozen
      and not banned and failed == 0 and passed >= 233
      and magic_ok and distinct_ok)
res = "PASS" if ok else "FAIL"
tok_lines = [f"- {k}: {'ok' if v else 'MISSING'}" for k, v in tokens.items()]
out = [
    f"# Typo-U report ({rounddir.name})",
    *tok_lines,
    f"- paint-neutral rich() 0.0 tracking x2: {'ok' if paint_neutral else 'FAIL'}",
    f"- frozen removals: {len(removed_frozen)} {'ok' if not removed_frozen else 'FAIL'}",
    f"- banned in non-test code: {len(banned)} {'ok' if not banned else 'FAIL ' + str(banned[:3])}",
    f"- cargo test: {passed} passed, {failed} failed (expect >=233/0)",
    f"- PNG: {len(png)} bytes, magic {'ok' if magic_ok else 'FAIL'}, distinct {distinct} ({'ok' if distinct_ok else 'FAIL'}), bit-identical to zoom-r baseline: {'yes' if identical else 'no (info)'}",
    "",
    f"RESULT: {res}",
]
(rounddir / "typo-u-report.md").write_text("\n".join(out) + "\n")
print("\n".join(out))
sys.exit(0 if res == "PASS" else 1)
PY
