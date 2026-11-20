# round-gui-exq6 round-1 — table bars wrap at narrow widths (geometry) + typo untouched

Scope: leaf-3.2 Exquisite GUI, geometry FIRST, typography SECOND. Colors FROZEN.
OWNS: crates/gui/src/app.rs, viewmodel.rs, views_hex.rs, views_table.rs,
checks/ac-loop/round-gui-exq6/**. No other paths touched.

## Geometry (first)
- Fix: 4 table bars in `crates/gui/src/views_table.rs` switched from
  `ui.horizontal` to `ui.horizontal_wrapped` (hex `paint_hex_find_bar`
  precedent reuse, no new tokens):
  - `paint_table_key_bar` (TABLE_KEY_BAR_H row, N toggles + badge)
  - `paint_table_find_bar` (TABLE_FIND_BAR_H row, needle 120px + Prev/Next + badge)
  - `paint_table_toolbar` (TABLE_FIND_BAR_H row, Find + page Prev/Next + label)
  - `paint_table_sheet_bar` (TABLE_SHEET_BAR_H row, N sheets + badge)
- Why: at the 720px narrow fallback each dual pane is 340px
  (`dual_pane_widths(720, 0.5) = 340/340`, chrome 40). A 10-column key bar
  plus needle/buttons/badge exceeds 340px in one row and clips; wrapped rows
  fold instead. At canonical 960px (panes 460/460) no wrap triggers, layout
  bit-identical. Caller signatures stable, `views.rs` untouched.
- Zero-clipped-text: bars now wrap rather than paint past the pane edge;
  badge stays right-end with existing PAD tokens, `set_min_size` bar heights
  unchanged (28.0), gaps unchanged (8.0/4.0).

## Typography (second)
- Unchanged: section labels Body-strong, cells Mono (+strong selected/key),
  badges/hints Small-role, needle Mono family. No new sizes, weights, or
  letter-spacing. Hierarchy label(13-strong) > content(12) > hint(11-weak)
  preserved.

## Edge fixture (comparison/)
- `comparison/edge-exq6-widetable-left.csv` vs `-right.csv`: 10-column header
  + 4 rows, single-cell delta (row 2 price 19.50 -> 21.00). Exercises the
  wide key-bar toggle row + toolbar at 720px: keyed diff, badge `key: ...`,
  no new wording.

## Gates (measured)
- GUI tests: `CARGO_TARGET_DIR=/tmp/target-op-guiexq6 CARGO_BUILD_JOBS=2
  cargo test -p autocompare-gui --lib` -> 728 passed, 0 failed.
- Clippy: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` ->
  Finished, zero warnings.
- Fmt: `cargo fmt -p autocompare-gui -- --check` -> ok.
- Banned-scan (non-test `crates/gui/src`): unwrap/expect/panic/todo/
  unimplemented/unreachable -> 0.
- Colors in `crates/gui` diff: `Color32::from_rgb` / `Visuals::` added -> 0.
- Diff: 4 insertions, 4 deletions, 1 file (`views_table.rs`).

RESULT: PASS
