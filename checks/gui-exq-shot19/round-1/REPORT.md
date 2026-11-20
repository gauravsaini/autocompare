# REPORT — wave shot19 (fixture typo pixels, round-1)

Scope: fixture-renderer mirror of typo17 live text typography — line-number
column with left inset sized from char metrics, row height from line-height
rhythm with breathing room (no glued rows), stable baselines; shot15 center
gutter + yellow hunk markers + shot18 chrome intact. Geometry FIRST,
typography SECOND (no delta), colors FROZEN (33 touches, unchanged).

## Touched functions (all in `crates/gui/src/screenshot.rs`, fixture rows only)

NEW (non-test paint helpers, pure):
- `fixture_text_row_h` — live `views::text_typo17_row_height` (17.6)
  rounded to whole fixture pixels = 18 (was hardcoded 24).
- `fixture_text_linenum_col_w` — live
  `views::text_linenum_column_width_from_metrics` over
  `gutter::gutter_linenum_digits`, rounded (7 rows -> 15, 1200 rows -> 37).
- `fixture_text_inner_pad` — live `views::text_row_inner_pad_px` rounded = 4.
- `fixture_text_x_off` — metric column + one pad of air (7 rows -> 19,
  was hardcoded 32).
- `fixture_text_baseline_dy` — centers the 7px glyph block in `row_h`
  (`(18-7)/2 = 5`, was hardcoded 6); same centering idea as live
  `views::text_row_baseline_offset_px`, restated in fixture pixels.
- `FIXTURE_GLYPH_H` (const = 7) — single source tracking `draw_glyph`
  rows 0..7.

CHANGED (non-test paint):
- `render_text_fixture_png` — row loop rewire only: `row_h` 24 ->
  `fixture_text_row_h()` (18), number x `+4` -> `+num_x_off` (4),
  text/accent x `+32` -> `+text_x_off` (19), text/accent/marker y `+6` ->
  `+base_dy` (5). Accent-block fills ride the same `row_h`/baseline;
  center `=>` markers ride the same baseline (x math untouched).
  Tabstrip/toolbar/pathmeta/pane_top/gutter paint untouched.

UNTOUCHED (proves parity by staying):
- `paint_topchrome_toolbar`, `paint_chrome`, `paint_pathmeta_strip`,
  `paint_segmented_status` — shot18 chrome intact (pane_top 174 holds).
- `text_fixture_center_layout` — still `(12,448,464,20,488,448)`;
  `TEXT_CENTER_MARK` (`=>`, `CHANGED_FG` reuse) — shot15 intact.
- `render_narrow_text_fixture_png`, `render_text_accent_fixture_png` and
  all other routes — untouched (ponytail: no diff where parity holds).
- `views.rs` — never touched (parallel wave owns it).

TESTS (same file, `#[cfg(test)]` only):
- `shot19_typotall_numbers_aligned` (new, green) — mirrors
  `comparison/shot-typo-tall-1200.json`.

## Evidence

- Baseline (pre-edit): harness `RESULT: PASS`, PNG 10708 B magic-ok
  distinct-256, suite 1040 passed / 0 failed; `before.png` copied pre-edit;
  pxdiff self-compare `VERDICT: IDENTICAL`.
- Post-change: harness `RESULT: PASS` (PNG 10703 B, magic ok, distinct 256;
  geometry: SPACE LG/MD/SM/XL/XS, margin/gap x608, Grid x16, row-h x327,
  width x85, gutter x1720; typo unchanged; colors 33).
- Suite: `cargo test -p autocompare-gui` -> 1041 passed / 0 failed
  (1040 + 1 new `shot19_typotall_numbers_aligned`).
- `cargo fmt -p autocompare-gui -- --check` -> clean (0).
- Banned-token grep on all added lines
  (unwrap/expect/panic/todo/unreachable/indexing) -> empty (ZERO-BANNED).
- Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
- Edge fixture: `comparison/shot-typo-tall-1200.json` (1200px tall:
  digits 4, fixture col 37, row_h 18, baseline 5, 66 visible rows with
  strictly monotonic tops, widest number fits column; center layout
  (12,448,464,20,488,448) + pane_top 174 intact).
- Pixel gate (verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 97239 of 576000
diff_bbox: (12,179)-(935,341)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Bbox honesty: x 12..935 covers both panes AND center column 464..484
  AND hunk rows (changed y210, deleted y228, inserted y246); chrome y~40
  + overview gutter pixel-identical (intact = zero diff there by design).
