# GATES — wave legend25 (live egui bottom legend, round-1)

OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-legend25/**` ONLY.
NEVER touch `crates/sessions/*`, `crates/gui/src/screenshot.rs`,
`crates/gui/src/theme.rs`, never `git add -A`.
Colors FROZEN (repo count must stay exactly 33).
Clean-room: reference behavior described in own words only; segment words
reuse this repo's own status12/status20 fixture phrasing, never BC strings.

Scope: mirror the status20 fixture bottom strips in the LIVE egui bottom bar.
Reference shows per-pane status labels (cursor position left, format word
middle, result summary right) plus a thin bottom legend strip with 4 segments
in equal cells. Ours paints per-pane `L<n> | preview` detail lines and a
flowing text-only legend AFTER the panes row — and the panes row consumes the
whole remaining viewport, so both strips sit below the fold (live bottom =
hatch/empty gap). Fix geometry FIRST (reserve the 34.4px bottom stack from the
panes height; 4 equal legend cells with marker ink + dividers), typography
SECOND (existing roles only), colors LAST (frozen).
Keep toolbar22 glyphs, gutter23 track/markers, filebar/mirror chrome intact.

## G1 — Baseline green (pre-edit)

- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-legend25 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-legend25/round-1 crates/gui/src` + `cp round-1/after.png round-1/before.png` + `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; `after.png` magic ok, >= 8192 B, distinct > 64; pxdiff self-compare `VERDICT: IDENTICAL` (proves wiring only).
- EVIDENCE (verbatim):
```
# Exquisite report (round-1)
- PNG: 11213 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x611, Grid x16, row-h x327, width calls x85, gutter x1753
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
```

## G2 — OWNS-only diff

- CHECK: `git status --short` + `git diff --name-only`
- EXPECT: only `checks/gui-exq-legend25/**` + `crates/gui/src/views.rs`; no `screenshot.rs`/`theme.rs`/`sessions/*`. REPORT.md lists every touched fn.

## G3 — Geometry FIRST (bottom stack reserved + visible)

- CHECK: `git diff -- crates/gui/src/views.rs`
- EXPECT: minimal delta:
  + new pure `text_bottom_stack_height_px()` (= pane status 15.2 + legend gap 4.0 + legend 15.2 = 34.4, status12 `status_bottom_stack_h` parity);
  + new pure `text_panes_height_reserved(avail_h)` (bottom stack off `avail_h`, then existing `text_panes_height` floor);
  + `paint_text_body` panes budget switches to the reserved form so per-pane strips + legend sit inside the viewport instead of below the fold;
  + per-pane labels become cursor/format/counts lines via existing `viewmodel::path_segmented_status_for` + `path_segmented_status_line` (honest `-` fallback, status20 `format_pane_status_line` rhythm) with new pure `text_pane_format_word()` (`Text: exact`, status20 fixture parity) + `text_pane_result_word(rows, is_left)` (equal/changed/deleted|inserted counts, zero parts omitted);
  + legend bar becomes 4 equal cells (`text_legend_cell_width` = avail/4) each with marker swatch (`text_legend_marker_px` = SPACE_SM 8, `text_legend_marker_ink` = CHANGED_BG/INSERTED_BG/DELETED_BG/track reuse) + label + 1px dim dividers at cell bounds;
  + toolbar22/gutter23/filebar paint sites untouched; no new deps, no new sizes, colors frozen.
- EVIDENCE: pasted `git diff --stat` + fn list into REPORT.md after run.

## G4 — Typography SECOND, minimal

- CHECK: `git diff -- crates/gui/src/views.rs | grep -E "^\+" | grep -E "\.size\(|FontId|letter_spacing" || echo TYPO-CLEAN`
- EXPECT: `TYPO-CLEAN` (legend text reuses `text_legend_role` + `resolved_font_id`; pane lines keep Mono + truncate; no new sizes/weights/families).

## G5 — Exquisite re-run PASS + fixture pxdiff ABANDONED with reason

- CHECK: re-run harness into same round dir; `python3 tools/exquisite/pxdiff.py before.png after.png`
- EXPECT: harness `RESULT: PASS`; pxdiff `VERDICT: IDENTICAL` (egui-only change, fixture renderer untouched) — mark ABANDONED with reason "egui-only paint change; fixture PNG cannot move".
- EVIDENCE (verbatim post-run):
```
# Exquisite report (round-1)
- PNG: 11213 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x615, Grid x16, row-h x327, width calls x85, gutter x1754
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
before: 960x600 ch=4
after: 960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
pxdiff exit: 0
```
  ABANDONED with reason: egui-only paint change (`crates/gui/src/views.rs` only, fixture renderer untouched) — fixture PNG cannot move.

## G6 — LIVE-PIXEL GATE (MANDATORY)

- CHECK: build, launch `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text` (CARGO_TARGET_DIR=/tmp/target-exq-legend25 build), sleep 6, `screencapture -x -o round-1/live.png`, kill GUI; then `uv run --with pillow` script crops the bottom bar band and ASSERTs (a) per-pane label ink present in left+right halves, (b) 4 legend cells each carry marker ink distinct vs bar face.
- EXPECT: both asserts hold; script output pasted here + REPORT.md; crop saved `round-1/live-crop.png`; gate passes only if both hold.
- EVIDENCE: pasted verbatim after run (see LIVE-PIXEL GATE OUTPUT below).

## G7 — Edge fixture + headless test (narrow 720 legend no overlap)

- CHECK: `round-1/comparison/legend-narrow-720.json` (720px: 4 legend cells 180px each, dividers at 180/360/540, labels truncate inside cell budget, per-pane halves 358px each, legend never overlaps pane content) + one headless test named `legend25_narrow720_no_overlap`.
- EXPECT: fixture exists; test green.

## G8 — MS Rust + touch/scope + colors

- CHECK: `#![forbid(unsafe_code)]` holds; no `unwrap/expect/panic/todo/unreachable/indexing` in non-test delta; GUI stays thin client; `cargo fmt` on touched files; `git status` shows ONLY `crates/gui/src/views.rs` + `checks/gui-exq-legend25/**`; NEVER touch `crates/sessions/*`, never `git add -A`; colors `Color32::from_rgb|Visuals::` == 33.
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; banned-token grep empty; colors 33.

## LIVE-PIXEL GATE OUTPUT (round-1, pasted verbatim)

```
live.png: 2880x1800
ink 0 (122, 102, 20): 816 matching pixels
ink 0: 1 swatch-sized candidates
ink 2 (24, 90, 46): 64 matching pixels
ink 2: 1 swatch-sized candidates
ink 3 (120, 32, 32): 65 matching pixels
ink 3: 1 swatch-sized candidates
ink 0: swatch x[516,524] y[1420,1428]
ink 2: swatch x[1476,1484] y[1420,1428]
ink 3: swatch x[1956,1964] y[1420,1428]
legend band y ~ 1424
ink 1 (gray): swatch x[996,1004]
legend cell centers x: [520, 1000, 1480, 1960]
ink 0: ring distinct=56 dark-face=56
ink 1: ring distinct=56 dark-face=56
ink 2: ring distinct=56 dark-face=56
ink 3: ring distinct=56 dark-face=56
gate (b): 4 legend cells each carry marker ink distinct vs bar face
strip row y ~ 1385 (peak grayish-bright=128)
legend span x [516,1964] mid 1240
gate (a): left-half label ink=522 right-half label ink=221
gate (a): per-pane label ink present in left+right cells
crop saved: checks/gui-exq-legend25/round-1/live-crop.png (3136, 226)
LIVE-PIXEL GATE: PASS
```

Gate passes: (a) 522/221 >= 30 both halves, (b) 4/4 cells ring-distinct with dark face. Script: `round-1/live-pixel-check.py` (run via `uv run --with pillow`). Zoomed crop: `round-1/live-crop.png` (3136x226).
