# REPORT — wave table30 (live table geometry, round-1)

Scope: Table comparison geometry parity with key-columns, sort indicators, numeric tolerance,
and xlsx sheet selector at 1200px wide without horizontal scrolling.
Geometry FIRST, typography SECOND, colors FROZEN.

## Touched functions (all in `crates/gui/src/views_table.rs`)

NEW (pure non-test helpers):
- `table30_avail_w` — 1176.0px available width for 1200px canvas (canvas minus two 12.0px origin margins).
- `table30_grid_origin` — (12.0, 12.0) origin reuse.
- `table30_col_spacing` — column spacing (8.0px = `SPACE_SM`).
- `table30_grid_gap_y` — row gap (4.0px = `SPACE_XS`).
- `table30_row_sym_pad` — row symmetric pad (4.0px = `SPACE_XS`).
- `table30_cell_pad_x` — cell padding (8.0px = `SPACE_SM`).
- `table30_header_h` — Heading line-height lock (21.6px).
- `table30_row_h` — Mono line-height lock (15.6px).
- `table30_header_body_delta` — 6.0px delta.
- `table30_header_sep_y` — 35.6px separator y.
- `table30_body_first_y` — 37.6px first body row y.
- `table30_header_underline_w` — 1.0px header underline.
- `table30_header_weight_ok` ��� 600 weight lock.
- `table30_col_widths_px` — outer pixel widths from character metrics plus cell padding.
- `table30_grid_total_w` — total grid width including col gaps.
- `table30_column_grid_fit` — fit gate ensuring no horizontal scroll (`needs_hscroll: false`).
- `table30_col_xs` — column x positions.
- `table30_separator_xs` — vertical separator x positions.
- `table30_col_aligns` — column alignment (numeric right, text left).
- `table30_cell_text_x` — text placement x position.
- `table30_key_widths_chars`, `table30_key_outer_widths`, `table30_key_col_xs` — key-column dimensions and placement.
- `table30_key_box_h`, `table30_key_box_rects` — key-column bounding box geometry.
- `table30_sort_marker`, `table30_is_sorted_col`, `table30_header_label`, `table30_sort_glyph_rect` — sort indicators and glyph placement.
- `table30_sanitize_tolerance`, `table30_tolerance_badge_label` — numeric tolerance handling and badge display.
- `table30_source_kind`, `table30_source_badge`, `table30_sheet_count`, `table30_sheet_clamp`, `table30_sheet_label`, `table30_sheet_badge` — multi-sheet xlsx workbook selector affordance.
- `table30_visible_rows` — visible rows at given pane height.

UNTOUCHED:
- `views.rs`, `screenshot.rs`, `viewmodel.rs`, `theme.rs` — NEVER touched.

TESTS (in `crates/gui/src/views_table.rs`):
- `table_exq_table30_round1_wide_cols_1200` — headless edge test proving `wide-cols-1200.json`.

## Evidence

- Harness: `tools/exquisite/run.sh checks/gui-exq-table30/round-1` exits 0 with `RESULT: PASS`.
- Edge fixture: `comparison/wide-cols-1200.json` verified with headless unit test.
- Zero banned tokens (`unwrap/expect/panic/todo/unsafe`).
- `cargo fmt` clean.
