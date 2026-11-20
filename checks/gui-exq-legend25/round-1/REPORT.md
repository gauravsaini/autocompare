# REPORT — wave legend25 (live egui bottom legend, round-1)

Scope: mirror the status20 fixture bottom strips in the LIVE egui bottom bar.
Reference shows per-pane status labels (cursor left, format word middle, result
summary right) plus a thin bottom legend strip with 4 segments in equal cells.
Ours painted per-pane `L<n> | preview` detail lines and a flowing text-only
legend AFTER the panes row — and the panes row consumed the whole remaining
viewport, so both strips sat below the fold (live bottom = hatch/empty gap).
legend25 reserves the 34.4px bottom stack from the panes height and repaints
the legend as 4 equal cells with marker ink + dividers.
Shot15 center gutter + `=>` markers, shot18 chrome, shot19 typo rows, toolbar22
glyphs, gutter23 track/markers, filebar/mirror chrome intact.
Geometry FIRST, typography SECOND (no delta), colors FROZEN (repo count exactly
33).

## Touched functions (all in `crates/gui/src/views.rs`)

NEW (pure headless helpers):
- `text_bottom_stack_height_px` — 15.2 + 4.0 + 15.2 = 34.4 (status12
  `status_bottom_stack_h` parity).
- `text_panes_height_reserved` — bottom stack off `avail_h`, then existing
  `viewmodel::text_panes_height` floor.
- `text_pane_format_word` — `"Text: exact"` (status20 fixture parity, honest:
  live text compare is always exact; wrap/syntax are display prefs).
- `text_pane_result_word` — `equal/changed` counts both sides plus
  left `deleted` / right `inserted`; zero parts omitted (status20 rhythm:
  `2 equal, 1 changed, 1 deleted`); empty rows give `""`.
- `text_pane_status_side_line` — `cursor | format | result` via existing
  `viewmodel::cursor_label/cursor_pos_for` + `path_segmented_status_for` +
  `path_segmented_status_line` (honest `- | Text: exact` fallback, status20
  `format_pane_status_line` rhythm, no new join).
- `text_legend_cell_width` — `avail / 4` (960→240, 720→180), NaN-g-guarded.
- `text_legend_marker_px` — `SPACE_SM` 8.0 square (no new size).
- `text_legend_marker_ink` — frozen reuse: CHANGED_BG / `track_for(dark)` /
  INSERTED_BG / DELETED_BG (no new hue).
- `text_legend_cell_budget_chars` — text px budget / mono advance
  (720: 160px = 22 chars), guarded.
- `text_legend_cell_label` — full inside budget else `...` clip
  (fixture `clip_to_width` ellipsis precedent, single-row lock).

CHANGED (paint):
- `paint_text_body` — panes budget switches to `text_panes_height_reserved`
  (comment updated); strips move inside the viewport instead of below the fold.
- `paint_text_pane_status` — halves geometry/typo unchanged
  (`pane_status_half_width`, 15.2 bar, Mono + truncate); lines now
  `text_pane_status_side_line` per side (cursor/format/counts).
- `paint_text_legend_bar` — 4 equal cells: `allocate_exact_size` row, 1px dim
  dividers at cell bounds, 8px marker swatch, `painter.text` label
  (`text_legend_role` font, weak tone, budget-clipped). Old flowing
  `horizontal` + `separator` shape replaced.

UNTOUCHED:
- `paint_toolbar_action_buttons`/`paint_action_nav_row` glyphs (toolbar22),
  center-gutter track/markers (gutter23), file-bar/meta chrome, cursor strip,
  3-zone top status bar, minimap/viewport math, wrap-on/wrap-off lists.
- Old detail helpers (`text_pane_cursor_labels`, `text_pane_preview_side`,
  `text_pane_detail_label`, `text_pane_detail_budget`) kept as tested `pub`
  API (no dead code, no test churn).

TESTS (same file, `#[cfg(test)]` only):
- `legend25_narrow720_no_overlap` (new, green) — mirrors
  `comparison/legend-narrow-720.json`.

## Evidence

- Baseline (pre-edit): harness `RESULT: PASS`, PNG 11213 B magic-ok distinct-256,
  suite green; `before.png` copied pre-edit; pxdiff self-compare `VERDICT:
  IDENTICAL`.
- Post-change: harness `RESULT: PASS` (PNG 11213 B, magic ok, distinct 256;
  geometry: SPACE LG/MD/SM/XL/XS, margin/gap x615, Grid x16, row-h x327,
  width x85, gutter x1754; typo unchanged; colors 33).
- Fixture pxdiff post-change (`before.png` vs `after.png`):
```
before: 960x600 ch=4
after: 960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
pxdiff exit: 0
```
  ABANDONED with reason: egui-only paint change; fixture renderer untouched,
  so the fixture PNG cannot move.
- Suite: `cargo test -p autocompare-gui` -> 1049 passed / 0 failed.
- `cargo fmt -p autocompare-gui -- --check` -> clean (0).
- Banned-token grep on added non-test lines
  (`unwrap(`/`.expect(`/`panic!`/`todo!`/`unreachable!`) -> empty
  (`ZERO-BANNED`); no indexing in delta.
- Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
- Edge fixture: `comparison/legend-narrow-720.json` (720px: 4 legend cells
  180px each, dividers at 180/360/540, labels truncate inside the 22-char
  budget, halves 358px each, 34.4px stack reserved so the legend never overlaps
  pane content; toolbar22/gutter23/filebar intact).
- Live-pixel gate (MASS-verified on-device, `LIVE-PIXEL GATE: PASS`):
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
  Launch: `/tmp/target-exq-legend25/debug/autocompare-gui --open
  tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt
  --tab text`, sleep 6, frontmost via System Events, `screencapture -x -o
  round-1/live.png`, kill GUI. Script: `round-1/live-pixel-check.py`
  (`uv run --with pillow`); crop: `round-1/live-crop.png`.
  Note: display-profile shifts consts by <=20/channel (yellow renders
  (119,103,38), green (44,89,50)); script matches with tolerance 24 and
  requires swatch-sized clusters + 3-ink band agreement, so the pass is
  structural, not color-exact.
