# Wave AG-SPLITDRAG — REPORT (round-1)

Gap #1 paint half ka MODEL hissa: splitter drag-handle geometry ab headless test surface (`viewmodel.rs`) me hai.
Split fraction (`0.20..=0.80` clamp + drag delta) pehle se tha (Wave Y); is wave me uske upar handle hit-rect,
drag-delta-to-fraction delta, cursor affordance aur a11y label ke pure helpers aaye. `views.rs` paint agle wave me (scope BAHAR).

## Before -> After (file:line)

- `crates/gui/src/viewmodel.rs:956-1037` naya section "drag-handle geometry model" (paint wiring agle wave ki, isliye `#[allow(dead_code)] // wiring-pending`):
  `SPLIT_HANDLE_HIT_W` (8.0, `views::SPLIT_HIT_W` mirror, geometry only) + `split_handle_x_range`
  (avail+fraction se `(x0,x1)`, 8px strip centered, edges par pin, degenerate avail -> `(0,0)`, NaN fraction -> default)
  + `split_handle_contains_x` (degenerate avail/NaN par kabhi hit nahi) + `split_fraction_from_x`
  (absolute x se clamped fraction, kharab input par default) + `split_drag_delta_fraction`
  (`dx/avail` delta, `split_apply_drag` ke saath consistent, zero/NaN guard -> `0.0`)
  + `split_cursor_affordance` (`"resize-column"`, paint ke `ResizeColumn` se judta hai)
  + `split_accessibility_label` (`"Splitter: left pane N percent. Drag to resize between 20 and 80 percent."`).
- `split_apply_drag` untouched (drag-delta-to-fraction authority wahi hai); naye helpers uske upar bane hain.
- `views.rs` / `app.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` untouched.

## Tests

- `cargo test -p autocompare-gui`: 339 passed, 0 failed (baseline 333 + 6 naye).
- Naye 6 viewmodel tests (DoD clamp coverage):
  `split_handle_hit_width_matches_paint_strip` (8.0 paint strip match),
  `split_handle_x_range_centers_on_fraction` (800/0.50 -> 396..404; clamp edges; degenerate/NaN guards),
  `split_handle_contains_x_hits_only_strip` (center/edge hit, bahar miss, NaN/degenerate miss),
  `split_fraction_from_x_clamps_and_guards` (400/800 -> 0.50; bahar clamp; NaN/0-avail default),
  `split_drag_delta_fraction_guards_and_scales` (80/800 -> 0.10; `split_apply_drag` consistency; guards -> 0.0),
  `split_cursor_and_a11y_label_read_honestly` (resize-column; label me percent + 20..80 bounds).
- Purane split tests same values par pass (`split_fraction_clamps_to_drag_range`, `split_drag_delta_moves_fraction`).

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-splitdrag-ag/round-1`: RESULT: PASS
  (PNG 9624 bytes, magic ok, distinct 256; `.size` values sirf ['14.0'] — koi typo change nahi).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero warnings.
- `rustfmt --edition 2024 --check` (OWNS file `viewmodel.rs`): clean.
- Banned grep (non-test `unwrap(`/`expect(`/`panic!`/`todo!`/...): ZERO-BANNED
  (`unwrap_or`/`unwrap_or_default` allowed hain, wahi use hote hain); `unsafe` zero (workspace `forbid` kept).

## Scope note

- Koi color/typo edit nahi; KEVL files untouched (working tree me `checks/gui-chrome-ag/` dikha — dusre agent ka, haath nahi lagaya).
- `cargo fmt -- --check` workspace-level par out-of-scope files par diff de sakta hai;
  OWNS file project rustfmt se clean hai.
- Koi filesystem persistence nahi; fraction sirf run-time state.
- STATUS.md untouched, koi push nahi (local commit only).
