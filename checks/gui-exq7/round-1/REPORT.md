# gui-exq7 round-1 — Hex + Gutter geometry wave 7b

Scope (OWNS only): `crates/gui/src/views_hex.rs`, `crates/gui/src/gutter.rs`,
`checks/gui-exq7/**`. Colors FROZEN, no new deps, no paint wiring (headless pure fns).

## Geometry FIRST

- Sync panes: `hex_sync_visible_range` + `hex_sync_row_starts` share one
  `[top, end)` window; hex digits and ASCII iterate identical 16-aligned
  starts. `HexPager::visible_range` parity locked in-range; OOB clamps.
- 64-bit goto: `hex_goto_anchor` = `hex_offset_to_row_col` reuse, so the goto
  bar lands on the same grid the ruler labels. `hex_goto_row_start` saturates
  (`u64::MAX` -> last row, never wraps). `parse_hex_goto` handles
  `0x`/decimal + separators, `clamp_hex_goto` honest `None` OOB.
- Gutter virtualization: `hex_hunk_row_ranges` (hunk -> `[first, end)` rows,
  clamped) + `hex_gutter_geometry_fast` straight into
  `gutter_geometry_from_ranges` — O(hunks), no per-row vec. Fast == slow
  bands asserted on split + fused runs (`hex_wave7_gutter_fast_matches_row_wise_bands`).
  `gutter.rs` addition is only `gutter_geometry_from_ranges` + test (114 lines).
- Zero clipped: `HEX_GRID_ROW_CHARS = 80` (10 ruler + 2 pad + 49 hex cells +
  1 pad + 16 ASCII + 2 pipes); covered/uncovered/insertion/tail rows all render
  exactly 80 chars (`format_hex_grid_row_with_bytes` shape parity).
- 960x600 + narrow: `HEX_CANVAS_W/H = 960/600`,
  `HEX_STACK_BREAKPOINT_W = 720` (documented paint fallback precedent).
  `hex_layout_mode`: >=720 SideBySide, else Stacked. Chrome reserve reuses
  gutter bar + minimap strip + 2x `SPACE_SM` (52.0 with numbers);
  `hex_grid_avail_w(960) = 908.0 > 0`, narrow floors at 0 (scroll, no clip).

## Typography SECOND

- `hex_sync_pane_text_role` / `hex_ruler_text_role` = `TypoRole::Mono` reuse
  (family/weight/line-height/tabular asserted; rank Body > Mono > Small).
  Goto/find labels reuse Body-strong, values Mono, badge Small — no new
  size/token/color.

## Evidence

- `cargo test -p autocompare-gui`: 743 passed, 0 failed
  (`checks/gui-exq7/round-1/tests.out`).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean.
- Banned (`unwrap/expect/panic/todo/unimplemented/unreachable`, indexing) in
  non-test owned code: zero. `cargo fmt --check -p autocompare-gui`: clean.
- Colors: owned diff touches no `Color32`/`Visuals`/palette. Deps: none added.
- Edge fixture `checks/gui-exq7/comparison/hex-sync-tail.json` (48-byte left vs
  49-byte right, diff at 20 + tail insertion): total_rows 4, window
  [0,16,32,48], goto(47) -> row 2/col 15/ruler 32, fast==slow, 2 segments,
  80-char rows. Mirrored by `hex_wave7_edge_fixture_tail_insertion_parity`: PASS.

RESULT: PASS
