# Wave AS-HEXGUTTER — REPORT (round-1)

Hex gutter parity: hex view me text gutter precedent par offsets/row markers + click-to-jump.
`gutter.rs` helpers reuse, geometry first (wahi bar widths, wahi min-height/merge-gap, marker alignment
wahi row bands par), typography second (wahi mono role/size/source), colors frozen. Koi egui/I/O nahi —
paint wiring orchestrator ke paas, ye sirf headless math hai.

## Before -> After (file:line)

- `crates/gui/src/views_hex.rs:1094-1257` naya `Hex gutter parity` section (sab `#[allow(dead_code)]`,
  test-only until paint wiring lands):
  `hex_gutter_total_rows` (`hex_row_count` reuse) (`1100`),
  `hex_gutter_geometry` (`hex_row_has_diff` + `GutterGeometry`/`GUTTER_MIN_SEGMENT_H`/`GUTTER_MERGE_GAP` reuse,
  kind `Changed`) (`1110`), thin delegates `hex_gutter_bar_width` (`1159`),
  `hex_gutter_bar_width_for_font` (`1166`), `hex_gutter_text_role/size/line_height_source` (`1173/1180/1187`),
  `hex_gutter_row_at_y`/`segment_at_y`/`y_for_row` (`1194/1201/1208`), `hex_gutter_row_label`
  (`format_hex_offset(hex_row_start(row))` reuse) (`1217`), `hex_gutter_jump_target`
  (`gutter_row_at_y` + `hex_row_to_hunk`, forward phir backward snap) (`1227`).
- `crates/gui/src/gutter.rs` me koi change nahi (reuse hi kaafi tha, single-source precedent untouched).
- `crates/gui/src/views_hex.rs:1780-1921` 5 naye unit tests (neeche).

## Tests

- `cargo test -p autocompare-gui`: 481 passed, 0 failed
  (baseline 473 + 5 naye + 3 sibling tablekey agent ke; worktree shared hai).
- Naye (5):
  `hex_gutter_geometry_marks_diff_rows_and_merges` (2 diff rows -> 2 `Changed` segments, lagatar rows merge,
  khali/zero khali), `hex_gutter_width_and_typo_follow_text_precedent` (bar widths + typo role/size/source
  `gutter.rs` ke barabar), `hex_gutter_row_at_y_delegates_to_text_mapping` (band mapping + reject + inverse),
  `hex_gutter_jump_target_snaps_to_covering_hunk` (covering/forward/backward/khali/oor),
  `hex_gutter_row_labels_use_offset_ruler` (`row_label == format_hex_offset`, total_rows math).
- Purana same values par pass: `hex_gutter_jump_targets_covering_hunks`.

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-hexgutter-as/round-1`: RESULT: PASS
  (PNG 9624 bytes, magic ok, distinct 256; Grid x7, mono x66, `.size` sirf ['14.0'] — koi typo/color change nahi).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero warnings (apni 1 collapsible-if fix ke baad).
- `rustfmt --edition 2024 --check` (OWNS files `views_hex.rs` + `gutter.rs`): clean (`cargo fmt --` sirf OWNS par).
- Banned grep (non-test `unwrap(`/`expect(`/`panic!`/`todo!`/...): ZERO-BANNED.
- `forbid(unsafe_code)` kept (`views_hex.rs:18`, `gutter.rs:8`).

## Scope note

- Koi color/typo edit nahi; `views.rs`/`app.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`home.rs`/`theme.rs`/
  `screenshot.rs` untouched (read bhi avoid — sirf `theme.rs` token grep + `viewmodel.rs` kind grep).
  Worktree me dusre tablekey agent ka `views_table.rs` diff + `checks/gui-tablekey-as/**` untracked dikha —
  haath nahi lagaya, commit me include nahi (sirf OWNS paths staged).
- `gutter.rs` OWNS me tha par change ki zaroorat nahi padi (reuse hi kaafi tha), isliye diff sirf `views_hex.rs` + `checks/gui-hexgutter-as/**`.
- STATUS.md untouched, koi `stash`/`push` nahi (local commit only).
