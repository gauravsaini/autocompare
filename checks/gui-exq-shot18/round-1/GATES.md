# GATES — wave shot18 (fixture chrome pixels)

OWNS: `crates/gui/src/screenshot.rs` (fixture chrome ONLY) + `checks/gui-exq-shot18/**` ONLY. NEVER touch `views.rs` (parallel wave owns it), `app.rs`, `theme.rs`, or any other path.
Colors FROZEN (no `Color32`/`Visuals` delta; repo count must stay 33).
Clean-room: BC behavior described in own words, never copy strings/icons/assets.

## G1 — Baseline green (pre-edit, recorded 2026-11-21)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-shot18 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-shot18/round-1 crates/gui/src`
  + `cp round-1/after.png round-1/before.png`
  + `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; `after.png` magic ok, >= 8192 B, distinct > 64; pxdiff `VERDICT: IDENTICAL` (self-compare proves gate wiring, not a pass).
- EVIDENCE (verbatim):
```
# Exquisite report (round-1)
- PNG: 10449 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x603, Grid x16, row-h x315, width calls x85, gutter x1714
- TYPO: TypoRole x1049, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
pxdiff exit: 0
```
  NOTE: task text said "exit 1" for IDENTICAL, but `pxdiff.py` exits 0 on
  identical without `--expect-diff` (exit 2 only with `--expect-diff`);
  honest wiring proof above stands.

## G2 — OWNS-only diff

- CHECK: `git status --short`
  + `git diff --name-only | grep -vE "^(crates/gui/src/screenshot.rs|checks/gui-exq-shot18/)" || echo OWNS-CLEAN`
- EXPECT: only `checks/gui-exq-shot18/**` + `crates/gui/src/screenshot.rs`; no `views.rs`/other paths. REPORT lists every touched fn.

## G3 — Geometry FIRST (fixture chrome17 mirror, token reuse only)

- CHECK: `git diff -- crates/gui/src/screenshot.rs`
- EXPECT: minimal delta, all inside fixture chrome paint:
  + tabstrip: `render_text_fixture_png` tab boxes/labels shift to 8px inter-tab rhythm + 6px trailing air (SECTION_GAP/TAB_BOTTOM_GAP parity, no new consts);
  + toolbar: `paint_topchrome_toolbar` taller band (two stacked 24px rows + 8px inter-row gap = 56, SECTION_GAP rhythm) with button rhythm + second nav row; pane_top shifts 142 -> 174 to hold the band (4px air preserved);
  + file-bar: keep `paint_pathmeta_strip` field+meta rows (reuse file-label strings; second metadata line stays in own words) — heights already at parity, no new strings;
  + center gutter + `=>` markers untouched (shot15 contract holds);
  + NO new dependencies, NO new colors (frozen palette reuse only).

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/screenshot.rs | grep -E "^\+" | grep -E "\.size\(|RichText|font" || echo TYPO-CLEAN`
- EXPECT: no new sizes/weights/families; fixture keeps 5px font + existing `draw_string` pen (geometry proof only).

## G5 — Edge fixture + headless test (narrow 720 chrome)

- CHECK: `ls checks/gui-exq-shot18/round-1/comparison/shot-chrome-720.json`
  + `cargo test -p autocompare-gui shot18_chrome720_no_overlap`
- EXPECT: fixture exists (720 fallback: toolbar rows + file-bar halves + tabstrip hold without overlap); test green, asserting no-overlap at 720.

## G6 — Harness PASS + PNG valid + pixel gate

- CHECK: re-run harness (same command as G1)
  + `python3 -c "d=open('checks/gui-exq-shot18/round-1/after.png','rb').read(); print(len(d), d[:8].hex(), len(set(d)))"`
  + `python3 tools/exquisite/pxdiff.py checks/gui-exq-shot18/round-1/before.png checks/gui-exq-shot18/round-1/after.png --expect-diff; echo "pxdiff exit: $?"`
- EXPECT: harness `RESULT: PASS`; PNG magic `89504e470d0a1a0a`, >= 8192 B, distinct > 64; pxdiff exit 0 (`DIFFERENT`) with full output pasted below and into REPORT.md; bbox must still span the center column region (x-range covers the live 20px center ~464..484 so shot15 markers are not regressed).
- EVIDENCE (verbatim post-run):
```
# Exquisite report (round-1)
- PNG: 10708 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x318, width calls x85, gutter x1715
- TYPO: TypoRole x1049, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
10708 89504e470d0a1a0a 256
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 138850 of 576000
diff_bbox: (0,40)-(959,448)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Bbox 0..959 spans center 464..484; y 40..448 spans chrome + hunk rows.

## G7 — Suite green + fmt + banned-token + colors

- CHECK: `cargo test -p autocompare-gui 2>&1 | grep "test result"`
  + `cargo fmt -p autocompare-gui -- --check; echo "fmt:$?"`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/screenshot.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `grep -rn "Color32::from_rgb\|Visuals::" crates/gui/src/ | wc -l`
- EXPECT: full suite green (1038 + 1 new = 1039 passed / 0 failed); `fmt:0`; ZERO-BANNED in non-test delta; colors exactly 33; `#![forbid(unsafe_code)]` intact; no indexing in new non-test code.
