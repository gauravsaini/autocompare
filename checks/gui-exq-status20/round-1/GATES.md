# GATES — wave status20 (fixture status strips, round-1)

OWNS: `crates/gui/src/screenshot.rs` (fixture status strips ONLY) + `checks/gui-exq-status20/**` ONLY. NEVER touch `views.rs` (parallel wave owns it), `app.rs`, `theme.rs`, `gutter.rs`, or any other path.
Colors FROZEN (no new hues; repo count must stay exactly 33).
Clean-room: describe reference behavior in own words, never copy strings/assets.

Scope: fixture-renderer mirror of per-pane status strips + bottom legend strip.
BC has per-pane position/format strips above each pane bottom plus a bottom legend
strip with result/action words. In our fixture renderer, draw per-pane one-line strips
(cursor position, format word, result word — single-line honest form from fixture viewmodel)
plus a bottom legend strip with 3-4 segment words (same/different/legend words in own phrasing).
Keep shot15 center gutter/markers, shot18 chrome, shot19 typo rows intact.
Geometry FIRST (strip height, divider x-positions, baselines, padding), typography SECOND
(no new sizes/weights/families; fixture keeps 5px pen), colors FROZEN.

## G1 — Baseline green (pre-edit)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-status20 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-status20/round-1 crates/gui/src` + `cp round-1/after.png round-1/before.png` + `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; `after.png` magic ok, >= 8192 B, distinct > 64; pxdiff `VERDICT: IDENTICAL` (self-compare proves gate wiring, not a pass).
- EVIDENCE (verbatim):
```
# Exquisite report (round-1)
- PNG: 10703 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1720
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
pxdiff exit: 0
test result: ok. 1041 passed; 0 failed
```

## G2 — OWNS-only diff

- CHECK: `git status --short` + `git diff --name-only | grep -vE "^(crates/gui/src/screenshot.rs|checks/gui-exq-status20/)" || echo OWNS-CLEAN`
- EXPECT: only `checks/gui-exq-status20/**` + `crates/gui/src/screenshot.rs`; no `views.rs`/other paths. REPORT.md lists every touched fn.
- EVIDENCE: `git status --short` shows only `M crates/gui/src/screenshot.rs` + `?? checks/gui-exq-status20/` as owned changes (other dirty paths — `crates/sessions/**`, `checks/gui-sessionpaint-by/**` — pre-existed, never staged). `views.rs` untouched (0 matches). REPORT.md lists every touched fn.

## G3 — Geometry FIRST (per-pane status strips + bottom legend strip)

- CHECK: `git diff -- crates/gui/src/screenshot.rs`
- EXPECT: minimal delta, inside fixture status strip paint:
  + new pure helpers: `PANE_STATUS_H` (18), `format_pane_status_line`, `paint_pane_status_strip`, `fixture_legend_segments`, `paint_bottom_legend_strip`;
  + `render_text_fixture_png`: replace single truncated status bar with per-pane one-line strips (left: "3:1 | Text: exact | 4 equal, 1 changed, 1 deleted"; right: "3:1 | Text: exact | 4 equal, 1 changed, 1 inserted") placed above pane bottoms at y=542..560, plus bottom legend strip at y=570..600 with 4 segment words ("Differences", "Same", "Insert", "Overview");
  + shot15 center gutter layout `(12, 448, 464, 20, 488, 448)` and markers `=>`, shot18 chrome (tabstrip y40, toolbar 114..170, pane_top 174), and shot19 typo row rhythm (18px) remain intact;
  + no new deps, colors frozen.
- EVIDENCE: `git diff -- crates/gui/src/screenshot.rs` shows pure helpers + status rewiring in `render_text_fixture_png` + `status20_narrow720_no_overlap` test only; narrow/accent/chrome/paint fns untouched.

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/screenshot.rs | grep -E "^\+" | grep -E "\.size\(|RichText|font" || echo TYPO-CLEAN`
- EXPECT: no new sizes/weights/families; fixture keeps 5px pen + existing `draw_string` and `clip_to_width`.
- EVIDENCE: `TYPO-CLEAN` (no added `.size(`/`RichText`/`font`).

## G5 — Edge fixture + headless test (narrow 720 status strips no overlap)

- CHECK: `ls checks/gui-exq-status20/round-1/comparison/status-narrow-720.json` + `cargo test -p autocompare-gui status20_narrow720_no_overlap`
- EXPECT: fixture exists (narrow 720px: per-pane strips fit within 340px panes without overlap across 4px gap; bottom legend 4 zones scale/clip cleanly); test green.
- EVIDENCE: `test screenshot::tests::status20_narrow720_no_overlap ... ok` (mirrors `comparison/status-narrow-720.json`: 340px panes, 4px gap, no overlap, 180px legend zones).

## G6 — Harness PASS + PNG valid + pixel gate

- CHECK: re-run harness `CARGO_TARGET_DIR=/tmp/target-exq-status20 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-status20/round-1 crates/gui/src` + `python3 -c "d=open('checks/gui-exq-status20/round-1/after.png','rb').read(); print(len(d), d[:8].hex(), len(set(d)))"` + `python3 tools/exquisite/pxdiff.py checks/gui-exq-status20/round-1/before.png checks/gui-exq-status20/round-1/after.png --expect-diff; echo "pxdiff exit: $?"`
- EXPECT: harness `RESULT: PASS`; PNG magic `89504e470d0a1a0a`, >= 8192 B, distinct > 64; pxdiff exit 0 (`DIFFERENT`) with full output pasted below and into REPORT.md; diff covers new status strips; chrome/gutter-overview/text rows intact.
- EVIDENCE (verbatim post-run):
```
# Exquisite report (round-1)
- PNG: 11085 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1720
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
11085 89504e470d0a1a0a 256
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 17873 of 576000
diff_bbox: (0,542)-(959,584)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Bbox honesty: y-range 542..584 covers the new per-pane status strips (y=542..560) and the bottom legend strip (y=570..600); x-range 0..959 spans across canvas. Chrome at y~40, center gutter at x~464..484, and hunk rows at y~174..341 are pixel-identical (zero diff there = intact by the strongest proof).

## G7 — Suite green + fmt + banned-token + colors

- CHECK: `cargo test -p autocompare-gui 2>&1 | grep "test result"` + `cargo fmt -p autocompare-gui -- --check; echo "fmt:$?"` + banned-token check on delta + colors count check.
- EXPECT: all tests pass; fmt clean (0); zero banned tokens in non-test delta; repo colors count exactly 33.
- EVIDENCE (verbatim):
  + Suite: `test result: ok. 1042 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.15s`
  + `cargo fmt -p autocompare-gui -- --check`: clean (exit 0)
  + Banned-token grep on non-test delta: `ZERO-BANNED`
  + Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
