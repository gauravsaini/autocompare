# GATES text38 — center gutter geometry (written BEFORE implementing per $unlazy)

Base: 4a2af71. Worktree: /tmp/wt-text38.
OWNS: `crates/gui/src/gutter.rs`, `checks/gui-exq-text38/**` ONLY.
NO-TOUCH: other gui files (`views.rs`, `copy.rs`, `take.rs`, `screenshot.rs`, `viewmodel.rs`, `theme.rs`), `crates/sessions/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`. NEVER push.
COLORS FROZEN: zero Color32 / Visuals additions.
MS Rust: forbid unsafe, no unwrap/expect/panic/todo/unreachable/indexing in non-test code.

## Intent (clean-room, own words)
In twin-pane side-by-side text comparisons, the center gutter serves as a focused vertical
lane between editor panes. Visual balance demands that this center lane maintain an exact 20px
width aligned symmetrically at the canvas midpoint (`cx = avail_w / 2.0`). Directional copy
arrows within the lane must center both horizontally within the 20px column (leaving a uniform 2px
margin on each side for a 16px arrow box) and vertically on the diff hunk row. Text row geometry
must strictly preserve the monospace typographic rhythm of `TypoRole::Mono` (pitch 15.6px,
line-height 1.30, compact row height 20.0px) so line indicators and gutter glyphs align with editor
lines without phase drift. On wide 1200px canvases, the center lane and symmetric 578px panes fit
with zero horizontal scrolling, while under a compact 700px narrow window, symmetric 328px panes
accommodate the 20px gutter without clipping or requiring horizontal scroll bars.

## Gates & Checks (geometry first, typography second, colors frozen)

### G1: Baseline harness execution
- CHECK: `tools/exquisite/run.sh checks/gui-exq-text38/round-1` exits 0 and produces `after.png` + `exquisite-report.md`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`.

### G2: Center 20px lane alignment reusing SPACE_* tokens
- CHECK: Center lane width `text38_center_lane_w()` == `SPACE_LG + SPACE_XS` = 20.0.
- CHECK: Symmetric layout `text38_center_lane_layout(avail_w)` computes `(margin, pane, 20.0, pane, margin)` via `gutter_text_layout`.
- CHECK: Canvas center alignment: `text38_center_lane_cx(avail_w)` == `avail_w / 2.0` (600.0 at 1200px, 480.0 at 960px, 350.0 at 700px).
- CHECK: Alignment verification: `text38_center_lane_is_aligned(avail_w)` is true for symmetric canvas widths.
- EXPECT: Perfect horizontal midpoint lock across 1200px, 960px, and 700px without fractional rounding errors.

### G3: Copy arrows centered in 20px lane
- CHECK: Arrow box dimension `text38_copy_arrow_box()` == `SPACE_LG` (16.0).
- CHECK: Side padding `text38_copy_arrow_pad_x()` == `(20.0 - 16.0) / 2.0` = 2.0 (`GUTTER_MERGE_GAP`).
- CHECK: Centered rect `text38_copy_arrow_rect(cx, cy)` returns `(cx - 8.0, cy - 8.0, cx + 8.0, cy + 8.0)`.
- CHECK: Row placement `text38_copy_arrow_in_row(row_top, row_h, col_x, col_w)` centers the 16x16 box vertically in `row_h` and horizontally in `col_w`.
- CHECK: Hit testing `text38_copy_arrow_hit_contains(cx, cy, x, y)` tests inclusion within the centered box with symmetric slop.
- EXPECT: Symmetric 16x16 arrow box centered within 20px column; pure; unit-tested.

### G4: Row height TypoRole Mono rhythm
- CHECK: Monospace typographic role `text38_mono_role()` == `TypoRole::Mono`.
- CHECK: Mono line-height multiplier `text38_mono_line_height()` == `TYPO_LINE_HEIGHT_MONO` (1.30).
- CHECK: Measured mono line-height `text38_mono_line_height_px()` == `TypoRole::Mono.line_height_px()` (15.6).
- CHECK: Compact row height `text38_row_height_compact()` == `SPACE_LG + SPACE_XS` = 20.0.
- CHECK: Padded row height `text38_row_height_padded()` == `15.6 + 2 * GUTTER_TEXT_ROW_PAD` = 19.6.
- CHECK: Monospace row center `text38_row_center_y(row, total_rows, row_h)` and top `text38_row_top_y(row, total_rows, row_h)` map without drift.
- EXPECT: Typographic single source reuse; zero ad-hoc magic numbers; pure; unit-tested.

### G5: 1200px canvas no horizontal scroll + narrow700 edge
- CHECK: `text38_canvas_w()` == 1200.0, `text38_narrow_w()` == 700.0.
- CHECK: Available widths account for `GEO_OUTER_MARGIN` (12.0 on each side): `text38_avail_w()` == 1176.0, `text38_narrow_avail_w()` == 676.0.
- CHECK: Layout at 1200px: `(12.0, 578.0, 20.0, 578.0, 12.0)`, total == 1200.0, `needs_hscroll` == false.
- CHECK: Layout at 700px: `(12.0, 328.0, 20.0, 328.0, 12.0)`, total == 700.0, `needs_hscroll` == false.
- CHECK: Fixtures `checks/gui-exq-text38/round-1/comparison/text38-wide-1200.json` and `text38-narrow-700.json` both verify `needs_hscroll: false`.
- EXPECT: Headless test verifies fixture values match pure implementation without deviation.

### G6: Typography & Colors Frozen
- CHECK: Reuses `TypoRole::Mono` and `TypoRole::Small`; zero new font size literals.
- CHECK: Zero additions of `Color32` or `Visuals` in `crates/gui/src/gutter.rs`.

### G7: Headless tests & MS Rust Quality
- CHECK: `cargo test -p autocompare-gui --lib gutter::exq_text38_tests` passes all tests.
- CHECK: `cargo fmt --check` passes cleanly on `crates/gui/src/gutter.rs`.
- CHECK: Clippy clean: `cargo clippy -p autocompare-gui --lib -- -D warnings`.
- CHECK: Zero banned keywords (`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unsafe`, indexing `[i]`) in non-test code.
- CHECK: Final harness run `tools/exquisite/run.sh checks/gui-exq-text38/round-1` reports `RESULT: PASS`.
