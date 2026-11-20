# REPORT — wave filebar24 (fixture file-bar, round-1)

Scope: fixture-renderer file-bar parity against Beyond Compare reference.
Reference shows one file-bar row above each pane: a path field on the left
(truncated when long) plus a metadata segment (timestamp, byte size, format
word — honest fallback when stat fails). Our fixture file-bar was two fixed
380px fields at x12/x490 with a clipped meta line and no bar band — the right
field bled past the pane into the overview gutter and the left Browse button
intruded toward the center gap. Filebar24 extends the status20 per-pane strips
precedent upward to the file-bar (same layout language, no new invention).
Shot15 center gutter + yellow hunk markers, shot18 chrome, shot19 typo rows,
toolbar21 glyphs/chevrons, and status20 strips stay intact.
Geometry FIRST, typography SECOND (no delta), colors FROZEN (repo count exactly 33).

## Touched functions (all in `crates/gui/src/screenshot.rs`)

NEW (non-test paint helpers/consts, pure):
- `FILE_BAR_Y` (const = 76, reuses `PATH_FIELD_Y`) — file-bar band top.
- `FILE_BAR_H` (const = 36 = `PATH_FIELD_H` + `PATH_META_H`) — field row 24 + metadata line 12; band y76..112, 2px air above toolbar 114.
- `FILE_BAR_BROWSE_W` (const = 75) — existing Browse width, refit inside each bar.
- `format_file_bar_meta` — formats `timestamp | bytes | format word` via `viewmodel::format_path_pane_meta`; empty parts drop, all-empty yields `"-"` (honest fallback when stat fails, mirrors status20 `format_pane_status_line`).

CHANGED (non-test paint):
- `paint_pathmeta_strip` — rewired to per-pane file-bar rows: bar backgrounds span the true pane columns (wide: `text_fixture_center_layout` → bars (12,76,448,36)/(488,76,448,36); narrow: `dual_pane_widths`/`dual_pane_x_positions`), path field + Browse refit inside each bar (field 365 + Browse 75 at 960; 257 + 75 at 720), path ellipsis-clipped to field budget, caret + folder/box marks track the bar, metadata segment clipped to bar width with `"-"` fallback when empty.
- `render_text_fixture_png` — meta args become 3-part via `format_file_bar_meta` (left `"2026-01-02 03:04 | 128 B | Text: exact"`, right `"2026-01-02 03:05 | 132 B | Text: exact"`); toolbar call, pane_top 174, rows, gutter, status strips, legend unchanged.

UNTOUCHED:
- `paint_topchrome_toolbar` (row band 114..170, icon marks, chevrons, nav pair) — toolbar21 intact.
- `paint_chrome`, tabstrip, window controls — shot18 intact.
- `text_fixture_center_layout` still `(12, 448, 464, 20, 488, 448)`; `TEXT_CENTER_MARK` (`=>`, `CHANGED_FG`) — shot15 intact.
- `fixture_text_row_h` (18), line-number column math, diff row loops — shot19 intact.
- `format_pane_status_line`, `paint_pane_status_strip`, `fixture_legend_segments`, `paint_bottom_legend_strip` — status20 intact.
- `views.rs`, `theme.rs`, `sessions/*` — NEVER touched (parallel waves own them).

TESTS (same file, `#[cfg(test)]` only):
- `filebar24_narrow720_no_overlap` (new, green) — mirrors `comparison/filebar-narrow-720.json`.

## Evidence

- Baseline (pre-edit): harness `RESULT: PASS`, PNG 11254 B magic-ok distinct-256, suite green; `before.png` copied pre-edit; pxdiff self-compare `VERDICT: IDENTICAL`.
- Post-change: harness `RESULT: PASS` (PNG 11213 B, magic ok, distinct 256; geometry: SPACE LG/MD/SM/XL/XS, margin/gap x608, Grid x16, row-h x327, width x85, gutter x1753; typo unchanged; colors 33).
- Suite: `cargo test -p autocompare-gui` -> 1047 passed / 0 failed (baseline 1046 + 1 new `filebar24_narrow720_no_overlap`).
- `cargo fmt -p autocompare-gui -- --check` -> clean (0).
- Banned-token grep on all added non-test lines (unwrap/expect/panic/todo/unreachable/indexing) -> empty (`ZERO-BANNED`).
- Colors: `Color32::from_rgb|Visuals::` repo count exactly 33 (unchanged).
- Edge fixture: `comparison/filebar-narrow-720.json` (720px narrow: bars (12,76,340,36)/(356,76,340,36), 4px gap, fields (12,76,257,24)/(356,76,257,24), meta budget 324px, `"-"` fallback; shot15 center gutter + shot18 chrome intact).
- Pixel gate (verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 12958 of 576000
diff_bbox: (12,76)-(952,111)
VERDICT: DIFFERENT
pxdiff exit: 0
```
  Bbox honesty: y-range 76..111 covers exactly the new file-bar band (bar backgrounds y76..112, fields y76..100, metadata line y100..112); x-range 12..952 spans both per-pane bars. Toolbar band at y114..170, hunk rows at y174+, center gutter at x464..484, and status strips at y542+ are pixel-identical (zero diff there = intact by the strongest proof).
