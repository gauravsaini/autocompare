# Wave Y-SPLIT — REPORT (round-1)

Gap #1 (draggable splitter) ka math ab headless test surface me single-source hai.
Drag handle `paint_paths` / dual-tree / picture-SideBySide me pehle se tha (Wave R);
ye wave uske neeche wali pure fraction logic (`0.20..=0.80` clamp + drag delta) ko
`viewmodel.rs` me canonical banata hai, `views.rs` delegate karta hai, `app.rs` untouched.

## Before -> After (file:line)

- `crates/gui/src/viewmodel.rs:925-953` naya section "Pane split fraction":
  `SPLIT_MIN_FRACTION` / `SPLIT_MAX_FRACTION` / `SPLIT_DEFAULT_FRACTION` (0.20/0.80/0.50)
  + `clamp_split_fraction` + `split_apply_drag` (zero-avail guard samet). Pure, no egui.
- `crates/gui/src/views.rs:55-61` purani local const/fn defs hati ->
  `pub use crate::viewmodel::{SPLIT_DEFAULT_FRACTION, clamp_split_fraction, split_apply_drag}`.
  `App` ke saare paths (`views::SPLIT_DEFAULT_FRACTION`, `views::clamp_split_fraction`,
  `views::split_apply_drag`, `views::split_picture_widths`) same signature par chalte hain.
- `crates/gui/src/views.rs` tests me sirf const paths badle
  (`SPLIT_MIN_FRACTION` -> `viewmodel::SPLIT_MIN_FRACTION`, 9 sites); koi value change nahi.
- `split_field_widths` / `split_picture_widths` / `paint_split_handle` / `dual_tree_pane_widths`
  untouched (GEO tokens `MIN_HALF` / `SECTION_GAP` / `SPLIT_HIT_W` reuse, koi naya token nahi).

## Tests

- `cargo test -p autocompare-gui`: 275 passed, 0 failed (baseline 271 + 2 naye).
- Naye viewmodel tests (DoD clamp coverage):
  `split_fraction_clamps_to_drag_range` (0.50 passthrough; 0.0/-5.0 -> 0.20; 1.0/9.9 -> 0.80),
  `split_drag_delta_moves_fraction` (80/800 -> 0.60; overshoot clamp; zero-avail guard).
- Purane views/app split tests same values par pass (delegation, koi behavior change nahi):
  `split_field_widths_follow_fraction`, `split_picture_widths_sum_to_row`,
  `split_drag_delta_moves_fraction`, `dual_tree_panes_share_row_and_floor`,
  `split_fraction_defaults_centered_and_clamps`, `split_drag_moves_fraction_within_range`.

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-split-y/round-1`: RESULT: PASS
  (PNG 9624 bytes, magic ok, distinct 256; `.size` values sirf ['14.0'] — koi typo change nahi).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero warnings.
- `rustfmt --edition 2024 --check` (OWNS files): clean.
- Banned grep (non-test `unwrap(`/`expect(`/`panic!`/`todo!`/...): ZERO-BANNED
  (`unwrap_or`/`unwrap_or_default` allowed hain, wahi use hote hain).

## Scope note

- Koi color/typo edit nahi; `app.rs` / `theme.rs` / `screenshot.rs` / `tree.rs` untouched
  (working tree me unpar dusre agent ka kaam dikha to haath nahi lagaya).
- `cargo fmt -- --check` workspace-level par unhi out-of-scope files par diff deta hai;
  OWNS files project rustfmt se clean hain.
- Koi filesystem persistence nahi; fraction sirf run-time state.
- STATUS.md untouched, koi push nahi (local commit only).
