# GATES — exq-agyxswarm80 round-1 (take snapshot and side handling polish)

OWNS: `crates/gui/src/take.rs` + `checks/gui-exq-agyxswarm80/take/**` only.
Colors FROZEN: zero new `Color32`, zero palette/Visuals touches in take.rs.

## CHECK: baseline cargo test PASS (pre-edit)
EXPECT: `cargo test -p autocompare-gui` passes (1140 unit tests + 2 real_egui_shot tests ok).
  (Observed PASS before any take.rs edit.)

## CHECK: side handling polish — TakeSide indexing, opposites, and choice conversions
EXPECT: `TakeSide` provides `ALL`, `sides()`, `index()`, `from_index()`, `from_gui_choice()`,
  `from_diff_choice()`, `opposite()`, `short_code()`.
  Theme token reuse: `take_side_gap() == GEO_TIGHT_GAP` (4.0),
  `take_side_bar_h() == TAKE_BUTTON_H == SPACE_XL` (24.0),
  `take_side_button_pad() == TAKE_BUTTON_PAD_X == SPACE_SM` (8.0).
  Headless unit tests prove index/opposite symmetry and conversions.

## CHECK: snapshot handling polish — TakeSession snapshot capture, restore, and inspection
EXPECT: `TakeSession` exposes `side_at(id)` and `resolved_sides()`;
  `TakeSessionSnapshot` captures `cursor`, `conflict_count`, `unresolved_count`,
  `is_fully_resolved`, `resolutions`, `output_lines`, `output_text`,
  plus derived `resolution_fraction()`, `resolution_bar_width()`, `stepper_text()`.
  `TakeSession::snapshot()` and `TakeSession::restore_snapshot()` round-trip resolutions;
  `save_output_to_path()` writes output cleanly returning `Result<usize, std::io::Error>`.

## CHECK: tmp-only persistence safety
EXPECT: all snapshot/output filesystem tests write strictly to `std::env::temp_dir()`,
  never polluting workspace repository paths.

## CHECK: pure safe Rust + 0 banned constructs
EXPECT: `#![forbid(unsafe_code)]` enforced;
  `grep -nE "unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe " crates/gui/src/take.rs`
  finds 0 occurrences in non-test code;
  `cargo fmt --check` clean.

## CHECK: full test suite green
EXPECT: `cargo test -p autocompare-gui` all green with new `exq_agyxswarm80_take_tests`.

## CHECK: commit OWNS only, conventional English, NO push
EXPECT: git commit touches only `crates/gui/src/take.rs` and `checks/gui-exq-agyxswarm80/take/**`.
