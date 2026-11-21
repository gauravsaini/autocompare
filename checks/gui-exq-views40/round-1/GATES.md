# GATES views40 — twin text panes round-2 (written BEFORE implementing per $unlazy)

Base: 9e0b1db. Worktree: /tmp/wt-views40.
OWNS: `crates/gui/src/views.rs`, `checks/gui-exq-views40/**` ONLY.
NO-TOUCH: other gui files, `crates/sessions/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`. NEVER push.
COLORS FROZEN: zero Color32 / Visuals additions.
MS Rust: forbid unsafe, no unwrap/expect/panic/todo in non-test code.
Clean-room own words. English-only comments and commit message.

## Intent
Twin text panes read as two editors with one rhythm. Each pane keeps
symmetric inner padding from the SPACE scale, line numbers stay
right-aligned in a tabular column that widens by digit count, and each
hunk header keeps symmetric air above and below so groups scan as units.
At 1200px wide the twin panes plus center gutter fit with no horizontal
scroll; at the narrow 700px edge the panes stack full-width with no
horizontal scroll.

## Gates (geometry first, typo second, colors frozen)

### G1: Baseline harness
- CHECK: `tools/exquisite/run.sh checks/gui-exq-views40/round-1` exits 0, writes `after.png` + `exquisite-report.md` with `RESULT: PASS`.

### G2: Twin pane padding reuses SPACE_*
- CHECK: `text40_twin_pad_px()` == `SPACE_XS` (4.0) == `TIGHT_GAP`.
- CHECK: `text40_pane_gap_px()` == `SPACE_SM` (8.0) == `SECTION_GAP` == `text_center_divider_gap_px()`.
- EXPECT: symmetric inner air both sides of number column and code; no new literal.

### G3: Line-number gutter alignment
- CHECK: `text40_linenum_pad_px()` == `SPACE_XS` (4.0).
- CHECK: `text40_linenum_col_width(digits, char_w)` == `text_linenum_column_width_from_metrics(digits, char_w)` (digits * char_w + 2 * pad).
- CHECK: `text40_gutter_tabular()` is true via `text_gutter_numbers_tabular()`.
- CHECK: `gutter_text` widths align: same char count for equal-width numbers and blanks.
- EXPECT: right-aligned tabular numbers; hidden numbers narrow to ruler track.

### G4: Hunk header spacing
- CHECK: `text40_hunk_header_gap_px()` == `SPACE_SM` (8.0).
- CHECK: `text40_hunk_header_pad_px()` == `SPACE_XS` (4.0).
- CHECK: `text40_hunk_header_height_px()` == `HEADER_H` (28.0).
- EXPECT: symmetric air around each hunk header; header bar reuses toolbar height.

### G5: 1200px no h-scroll + narrow700 edge
- CHECK: `text40_wide1200_pane_pair()` == `text_pane_pair_widths(1200.0)` == (582.0, 582.0).
- CHECK: `text40_wide1200_total_w()` == 1200.0; `text40_wide1200_no_hscroll()` true.
- CHECK: `text40_narrow700_layout()` == `Stacked`; `text40_narrow700_stacked_width()` == `text_stacked_pane_width(700.0)` == 692.0; `text40_narrow700_no_hscroll()` true.
- CHECK: fixtures `comparison/views40-wide-1200.json` and `comparison/views40-narrow-700.json` lock these values; headless test proves them.
- EXPECT: no horizontal scroll at either width.

### G6: Typo + colors frozen
- CHECK: reuses `TypoRole::Mono` rhythm only; zero new sizes/weights.
- CHECK: zero `Color32`/`Visuals` additions in `views.rs` diff.

### G7: Quality
- CHECK: `cargo test -p autocompare-gui` passes; new `exq_views40_tests` module passes.
- CHECK: `cargo fmt --check` clean; banned scan (`unwrap|expect|panic!|todo!|unimplemented!|unsafe`) 0 in non-test code.
- CHECK: re-run `tools/exquisite/run.sh checks/gui-exq-views40/round-1` reports `RESULT: PASS`.
