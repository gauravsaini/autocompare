# GATES copy34 — live copy arrows geometry first, typography second (written BEFORE implementing per $unlazy)

OWNS: crates/gui/src/copy.rs, checks/gui-exq-copy34/**
NO-TOUCH: views.rs, screenshot.rs, viewmodel.rs, views_table.rs, views_hex.rs, tree.rs, home.rs, theme.rs. NEVER push.
COLORS FROZEN: zero Color32/Visuals additions.
MS Rust: forbid unsafe, no unwrap/expect/panic/todo/unreachable in non-test code.

## Intent (clean-room, own words)
In the comparative view, inline and gutter copy actions transfer differences between
left and right panes. Left and right directional copy arrows must align cleanly using
standard spacing tokens (`SPACE_*`), each copy button must have an unambiguous 16px
square hitbox centered vertically and horizontally on the row, inline preview rows
must strictly adhere to the `TypoRole::Mono` line-height rhythm (15.6px) so preview text
mirrors editor text without jitter, and a 1200px wide canvas must accommodate all copy
affordances without horizontal scrolling while a 700px narrow window keeps arrow controls
usable and aligned without clipping.

## Gates & Checks (geometry first, typography second, colors frozen)

### G1: Baseline harness execution
- CHECK: `tools/exquisite/run.sh checks/gui-exq-copy34/round-1` exits 0 and produces `after.png` + `exquisite-report.md`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`.

### G2: Left/right arrows alignment reusing SPACE_* tokens
- CHECK: Left and right arrow buttons align using `SPACE_*` tokens (`SPACE_XS` = 4.0, `SPACE_SM` = 8.0, `SPACE_MD` = 12.0, `SPACE_LG` = 16.0, `SPACE_XL` = 24.0).
- CHECK: Column width and padding derive from `SPACE_XL` (24.0 column width) and `SPACE_XS` (4.0 side padding): `(24.0 - 16.0) / 2.0 = 4.0`.
- EXPECT: `copy34_arrow_column_w()` == `SPACE_XL` (24.0); `copy34_arrow_pad_x()` == `SPACE_XS` (4.0); `copy34_arrow_gap()` == `SPACE_XS` (4.0).

### G3: Copy button hitbox 16px centered
- CHECK: `copy34_button_hitbox_size()` == `SPACE_LG` (16.0).
- CHECK: `copy34_button_hitbox_rect(cx, cy)` returns a 16x16 rect centered at `(cx, cy)`: `(cx - 8.0, cy - 8.0, cx + 8.0, cy + 8.0)`.
- CHECK: `copy34_button_hitbox_in_row(row_top, row_h, col_x, col_w)` centers the 16x16 hitbox in `(col_x, row_top, col_x + col_w, row_top + row_h)`.
- CHECK: `copy34_button_hitbox_contains(cx, cy, x, y)` tests inclusion within the 16x16 centered rect; non-finite coordinates return false.
- EXPECT: Symmetric 16x16 hitbox centered precisely; pure; unit-tested.

### G4: Inline preview row height reusing TypoRole::Mono
- CHECK: `copy34_inline_preview_role()` == `TypoRole::Mono`.
- CHECK: `copy34_inline_preview_row_height()` == `TypoRole::Mono.line_height_px()` (15.6px).
- CHECK: `copy34_inline_preview_block_height(n)` == `n as f32 * 15.6`.
- EXPECT: Single-source monospace line-height parity without ad-hoc magic numbers; pure; unit-tested.

### G5: 1200px canvas no horizontal scroll + narrow700 edge
- CHECK: `copy34_canvas_w()` == 1200.0, `copy34_narrow_w()` == 700.0.
- CHECK: Available widths account for `GEO_OUTER_MARGIN` (12.0 on each side): `copy34_avail_w()` == 1176.0, `copy34_narrow_avail_w()` == 676.0.
- CHECK: `copy34_needs_hscroll(avail_w, content_w)` is false when content fits available width.
- EXPECT: Fixture `checks/gui-exq-copy34/round-1/comparison/copy-wide-1200.json` and `copy-narrow-700.json` both verify `needs_hscroll: false`.

### G6: Typography & Colors Frozen
- CHECK: TypoRole reuse only (`TypoRole::Mono` for preview, `TypoRole::Small` for chrome status); zero new font size literals.
- CHECK: Zero additions of `Color32` or `Visuals` in `crates/gui/src/copy.rs`.

### G7: Headless tests & MS Rust Quality
- CHECK: `cargo test -p autocompare-gui --lib copy::copy34` passes all tests.
- CHECK: `cargo fmt --check` passes cleanly on `crates/gui/src/copy.rs`.
- CHECK: Clippy clean: `cargo clippy -p autocompare-gui --lib -- -D warnings`.
- CHECK: Zero banned keywords (`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unsafe`, indexing `[i]`) in non-test code.
- CHECK: Final harness run `tools/exquisite/run.sh checks/gui-exq-copy34/round-1` reports `RESULT: PASS`.
