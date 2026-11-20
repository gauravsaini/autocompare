# GATES — wave shot19 (fixture typo pixels, round-1)

OWNS: `crates/gui/src/screenshot.rs` (fixture rows ONLY) + `checks/gui-exq-shot19/**` ONLY. NEVER touch `views.rs` (parallel wave owns it), `app.rs`, `theme.rs`, `gutter.rs`, or any other path.
Colors FROZEN (no new hues; repo count must stay exactly 33).
Clean-room: describe reference behavior in own words, never copy strings/assets.

Scope: fixture-renderer mirror of typo17 live text typography — line-number
column with left inset sized from char metrics, row height from line-height
rhythm with breathing room (no glued rows), stable baselines; shot15 center
gutter + yellow hunk markers + shot18 chrome stay intact.
Geometry FIRST (row heights, column offsets, baselines), typography SECOND
(no new sizes/weights/families; fixture keeps 5px pen), colors FROZEN.

## G1 — Baseline green (pre-edit)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-shot19 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-shot19/round-1 crates/gui/src` + `cp round-1/after.png round-1/before.png` + `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; `after.png` magic ok, >= 8192 B, distinct > 64; pxdiff `VERDICT: IDENTICAL` (self-compare proves gate wiring, not a pass).
- EVIDENCE (verbatim):
```
# Exquisite report (round-1)
- PNG: 10708 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1717
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
pxdiff exit: 0
test result: ok. 1040 passed; 0 failed
```

## G2 — OWNS-only diff

- CHECK: `git status --short` + `git diff --name-only | grep -vE "^(crates/gui/src/screenshot.rs|checks/gui-exq-shot19/)" || echo OWNS-CLEAN`
- EXPECT: only `checks/gui-exq-shot19/**` + `crates/gui/src/screenshot.rs`; no `views.rs`/other paths. REPORT.md lists every touched fn.
- EVIDENCE: `git status --short` shows only `M crates/gui/src/screenshot.rs` + `?? checks/gui-exq-shot19/` as owned changes (other dirty paths — `crates/sessions/**`, `checks/gui-sessionpaint-by/**` — pre-existed, never staged). `views.rs` untouched (0 matches). REPORT.md lists every touched fn.

## G3 — Geometry FIRST (fixture typo17 mirror, token/helper reuse only)

- CHECK: `git diff -- crates/gui/src/screenshot.rs`
- EXPECT: minimal delta, all inside fixture text-row paint:
  + new tiny pure helpers reusing live `views::` metrics (`text_typo17_row_height`, `text_linenum_column_width_from_metrics`, `text_char_metric_width`, `text_row_inner_pad_px`) + `gutter::gutter_linenum_digits` — no new deps, no new hues;
  + `render_text_fixture_png` row loop: hardcoded `row_h 24` -> fixture rhythm `18` (live 17.6 rounded), hardcoded text offset `32` -> metrics column `pad(4)+col(15)=19`, hardcoded baseline `y+6` -> centered stable `y+5` (row_h 18, 7px glyph);
  + line numbers keep left inset `pad` (4); accent-block fills + center `=>` markers ride the same `row_h`/baseline; center layout `(12,448,464,20,488,448)` untouched;
  + shot18 chrome (`paint_topchrome_toolbar`, `paint_chrome`, `paint_pathmeta_strip`, `pane_top 174`) untouched; narrow/accent routes untouched.
- EVIDENCE: `git diff -- crates/gui/src/screenshot.rs` = 5 new pure helpers + `FIXTURE_GLYPH_H` const + 14-line row-loop rewire in `render_text_fixture_png` only; narrow/accent/chrome/paint fns untouched.

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/screenshot.rs | grep -E "^\+" | grep -E "\.size\(|RichText|font" || echo TYPO-CLEAN`
- EXPECT: no new sizes/weights/families; fixture keeps 5px pen + existing `draw_string` (geometry proof only).
- EVIDENCE: `TYPO-CLEAN` (no added `.size(`/`RichText`/`font`).

## G5 — Edge fixture + headless test (tall 1200 numbers-aligned)

- CHECK: `ls checks/gui-exq-shot19/round-1/comparison/shot-typo-tall-1200.json` + `cargo test -p autocompare-gui shot19_typotall_numbers_aligned`
- EXPECT: fixture exists (tall 1200px: numbers aligned, baselines stable, generous row height from line-height rhythm); test green.
- EVIDENCE: `test screenshot::tests::shot19_typotall_numbers_aligned ... ok` (mirrors `comparison/shot-typo-tall-1200.json`: col 37, row_h 18, 66 visible rows).

## G6 — Harness PASS + PNG valid + pixel gate

- CHECK: re-run harness (same command as G1) + `python3 -c "d=open('checks/gui-exq-shot19/round-1/after.png','rb').read(); print(len(d), d[:8].hex(), len(set(d)))"` + `python3 tools/exquisite/pxdiff.py checks/gui-exq-shot19/round-1/before.png checks/gui-exq-shot19/round-1/after.png --expect-diff; echo "pxdiff exit: $?"`
- EXPECT: harness `RESULT: PASS`; PNG magic `89504e470d0a1a0a`, >= 8192 B, distinct > 64; pxdiff exit 0 (`DIFFERENT`) with full output pasted below and into REPORT.md; bbox must still span center column x~464..484 AND hunk rows (shot15 markers intact); chrome/gutter-overview identical by design (intact = zero diff there).
- EVIDENCE (verbatim post-run):
```
# Exquisite report (round-1)
- PNG: 10703 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x608, Grid x16, row-h x327, width calls x85, gutter x1720
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
10703 89504e470d0a1a0a 256
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 97239 of 576000
diff_bbox: (12,179)-(935,341)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Honesty note on bbox: x-range 12..935 covers both panes AND the live
  20px center column (464..484, `=>` markers ride the new rhythm);
  y-range 179..341 covers all 7 rows incl. hunk rows (changed idx2 y210,
  deleted idx3, inserted idx4). Shot18 chrome (tabstrip y40, toolbar
  114..170, pane_top 174) + shot15 overview gutter are pixel-IDENTICAL
  (zero diff there = intact by the strongest proof; layout asserts for
  both still green in `shot18_chrome720_no_overlap` + the new test).
