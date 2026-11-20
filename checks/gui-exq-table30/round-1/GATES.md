# GATES table30 — table geometry: key-cols + sort + tolerance + xlsx sheet (written BEFORE implementing per leaf prompt)

OWNS: crates/gui/src/views_table.rs, checks/gui-exq-table30/**
NO-TOUCH: views.rs, screenshot.rs, viewmodel.rs, theme.rs, all other crates. NEVER push.

Scope: Table key-compare geometry parity wave. Live table view owns views_table.rs;
this wave locks the 1200px wide table geometry without horizontal scroll:
key columns selection + sort indicators + numeric tolerance + xlsx sheet selector,
column widths reusing SPACE_* tokens, Grid reuse, and zero horizontal scroll at 1200px canvas.
Geometry FIRST, typography SECOND, colors FROZEN.

Clean-room (own words): In the reference comparative tool, tabular data alignment
spans multiple columns with distinct roles: key columns define row identity across
tables, column headers provide ascending or descending sort toggles, comparison options
allow numeric tolerance thresholds for floating-point calculations, and workbook files
provide sheet switching tabs for multi-sheet spreadsheets. On a wide 1200px layout,
the table grid accommodates multi-column data without horizontal scroll overflow.
All column widths derive from character metrics with consistent spacing tokens,
maintaining aligned grid separators and clean row height rhythms.

## CHECK: baseline harness runs on fresh round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-table30/round-1` exits 0 and writes `after.png` + `exquisite-report.md` with `RESULT: PASS`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`.

## CHECK: 1200px canvas & no horizontal scroll geometry
- CHECK: `TABLE30_CANVAS_W` is 1200.0px, available width `table30_avail_w()` is 1176.0px (canvas minus two 12.0px origin margins).
- CHECK: `table30_column_grid_fit` verifies column outer widths fit within available width without horizontal scrolling (`needs_hscroll: false`).
- CHECK: column spacing reuses `crate::theme::SPACE_SM` (8.0px = `table_col_spacing()`); row gap reuses `crate::theme::SPACE_XS` (4.0px = `TABLE_GRID_ROW_GAP`).
- CHECK: row heights locked to TypoRole line-heights: header 21.6px (`table30_header_h()`), body 15.6px (`table30_row_h()`), header underline 1.0px, header weight 600 (semibold).
- EXPECT: full 1200px grid geometry functions are pure, total, and unit-tested.

## CHECK: key-cols affordance
- CHECK: `table30_key_widths_chars`, `table30_key_outer_widths`, `table30_key_col_xs`, and `table30_key_box_rects` provide key column dimensions and placement.
- EXPECT: key columns calculate metrics accurately, out-of-range keys are safely skipped, and boxes enclose all rows without panicking.

## CHECK: sort affordance
- CHECK: `table30_sort_marker` returns "▲" (asc), "▼" (desc), or " " (unsorted); `table30_is_sorted_col` checks active sort; `table30_header_label` appends sort marker; `table30_sort_glyph_rect` computes glyph placement slot.
- EXPECT: single-character fixed indicator width, aligned with column boundaries.

## CHECK: tolerance affordance
- CHECK: `table30_tolerance_badge_label` provides status label for numeric tolerance (e.g. "tol: 0.05" or "exact text"); `table30_sanitize_tolerance` delegates to engine-safe finite non-negative values.
- EXPECT: non-finite or negative values sanitize to exact matching (`None`).

## CHECK: xlsx sheet selector affordance
- CHECK: `table30_sheet_count`, `table30_sheet_clamp`, `table30_sheet_label`, and `table30_sheet_badge` handle multi-sheet workbook navigation.
- EXPECT: clamp bounds indices safely, badge reflects "xlsx" / "csv" / "tsv" source kind cleanly.

## CHECK: edge wide-cols-1200 fixture + headless test
- CHECK: create `checks/gui-exq-table30/comparison/wide-cols-1200.json` and `checks/gui-exq-table30/round-1/comparison/wide-cols-1200.json` with 1200px canvas, key-cols, sort, tolerance, xlsx sheet, and `needs_hscroll: false`.
- CHECK: headless test `table_exq_table30_round1_wide_cols_1200` in `views_table.rs` verifies all geometry tokens, fixture parity via `include_str!`, and all table30 functions.
- EXPECT: test passes cleanly in `cargo test -p autocompare-gui`.

## CHECK: MS Rust + touch/scope + colors + fmt
- CHECK: `#![forbid(unsafe_code)]` holds; zero `unwrap/expect/panic/todo` in non-test code; touch ONLY `crates/gui/src/views_table.rs` and `checks/gui-exq-table30/**`.
- CHECK: `cargo fmt --check` clean; banned-token check clean.
- EXPECT: harness passes with `RESULT: PASS`.
