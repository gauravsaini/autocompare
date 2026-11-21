# GATES table43 — live table compare grid geometry lock (written BEFORE implementing per leaf prompt)

OWNS: crates/gui/src/views_table.rs, checks/gui-exq-table43/**
NO-TOUCH: views.rs, screenshot.rs, viewmodel.rs, theme.rs, all other crates/gui files, all other checks dirs. NEVER push.

Scope: live table compare grid geometry lock. Thin delegation layer over the
table30 single-source grid lock so live views cannot drift: 1200px canvas,
avail width, origin, col spacing/row gap, header/body rhythm, underline,
weight, char-metric outer widths, grid total, fit gate, col xs, separators,
grid column count, grid default, cell rect, visible rows.
Geometry FIRST, typography SECOND (underline/weight reuse only), colors FROZEN
(zero Color32/Visuals touches, zero new spacing/typo tokens).

Clean-room (own words): In the reference comparative tool, tabular data lines
up in a grid where each column keeps one stable width, header height stays
taller than body rows, separators sit halfway between columns, and the whole
grid fits the window width without sideways scrolling. A live compare grid
reuses the same widths, gaps, and row rhythm everywhere it paints so header,
body, and key highlights never drift apart.

## CHECK: baseline harness runs on fresh round-1 dir
- CHECK: `tools/exquisite/run.sh checks/gui-exq-table43/round-1` exits 0 and writes `after.png` + `exquisite-report.md` with `RESULT: PASS`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`.

## CHECK: live grid geometry delegates to single-source helpers
- CHECK: `cargo test -p autocompare-gui views_table::tests::table_exq_table43_live_table_grid` exits 0.
- EXPECT: test proves `table43_avail_w` 1176.0, origin (12.0,12.0), header 21.6, row 15.6, sep 35.6, body-first 37.6, col spacing 8.0, row gap 4.0, underline 1.0, weight ok, outer widths [32.0,48.0,40.0], total 136.0, fit true, xs [0.0,40.0,96.0], separators [36.0,92.0], grid columns 4, grid default origin/row/gaps, cell rect x/w/h, visible rows 38.

## CHECK: edge live-table-grid fixture + headless test
- CHECK: `checks/gui-exq-table43/comparison/live-table-grid.json` and `checks/gui-exq-table43/round-1/comparison/live-table-grid.json` exist with identical bytes and `"needs_hscroll": false`.
- EXPECT: headless test `table_exq_table43_live_table_grid` verifies fixture parity via `include_str!` plus all table43 functions.

## CHECK: MS Rust + touch/scope + colors + fmt
- CHECK: `#![forbid(unsafe_code)]` holds; zero `unwrap/expect/panic/todo/unreachable/unsafe` in non-test code (`grep -rn` clean); `git status --short` touches ONLY `crates/gui/src/views_table.rs` and `checks/gui-exq-table43/**`; zero `Color32/Visuals` additions; `cargo fmt --check` clean.
- EXPECT: harness re-run passes with `RESULT: PASS`.
