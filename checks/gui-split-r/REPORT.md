# Wave R SPLIT — REPORT (round-R-1)

Gap #1 (draggable splitter nahi) FIXED hai. Pehle do jagah static
halves the: path fields `half_field_width` se 50/50, picture columns
`picture_column_width` se 50/50. Ab dono ke beech drag handle hai aur
fraction App state me rehta hai (sirf run-time, koi file write nahi).

## Before -> After (file:line)

- `crates/gui/src/views.rs:56` `half_field_width` -> `:67` centered
  wrapper (ab `split_field_widths(avail, 0.50).0` par bana, value same:
  800 -> 325.0, 0 -> 105.0). Pehle wali static body hati.
- `crates/gui/src/views.rs:198` call site (paint_paths) -> `:260`
  `paint_paths(..., split: &mut f32)`: `split_field_widths` se widths
  + beech me `paint_split_handle` (Sense::drag).
- `crates/gui/src/views.rs:1509` `picture_column_width` -> `:1587`
  centered wrapper (ab `split_picture_widths(avail, 0.50).0` par bana,
  value same: 800 -> 396.0/396.0).
- `crates/gui/src/views.rs:1587` call site (SideBySide) -> `:1666`
  `split_picture_widths` + columns ke beech `paint_split_handle`.
- Naya math (views.rs): `:58-60` SPLIT_MIN/MAX/DEFAULT (0.20/0.80/0.50),
  `:77` `clamp_split_fraction`, `:84` `split_apply_drag`, `:94`
  `split_field_widths`, `:105` `split_picture_widths`, `:115`
  `paint_split_handle`.
- `crates/gui/src/app.rs:112` `App::split_fraction` field (per-run),
  `:167` default 0.50, `:460` getter, `:465` setter (clamp),
  `:473` `drag_split_by(dx, avail)`.
- 7 `paint_paths` call sites (`:2396`-`:2477` aas-paas) +
  1 picture site (`:2985` aas-paas) `&mut self.split_fraction` dete hain.
  `paint_triple_paths` (3-way) untouched hai.

## Tests

- `cargo test -p autocompare-gui`: 229 passed, 0 failed.
- Naye 6: views me 4 (`split_fraction_clamps_to_drag_range`,
  `split_field_widths_follow_fraction`, `split_picture_widths_sum_to_row`,
  `split_drag_delta_moves_fraction`) + app me 2
  (`split_fraction_defaults_centered_and_clamps`,
  `split_drag_moves_fraction_within_range`).
- Purane static-half tests same values par pass (wrappers single-source).

## Harness

- `tools/gui-polish/run-geo-typo.sh checks/gui-split-r/round-R-1`:
  RESULT: PASS (PNG 9914 bytes, magic ok, widths 13 calls, .size
  values sirf ['14.0'] — koi typo change nahi).
- `cargo fmt` (edition 2024, sirf views.rs + app.rs): clean.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`:
  views.rs + app.rs me zero errors. `theme.rs:673-674` par 2 errors
  hain jo parallel ZOOM wave ke adhure kaam ke hain — meri ownership
  ke bahar, isliye touch nahi kiya.

## Scope note

- Koi color/typo edit nahi (handle me koi naya hue ya `.size()` nahi;
  sirf existing visuals bg_fill).
- Koi filesystem persistence nahi; fraction sirf App struct me.
- STATUS.md untouched, koi push nahi (local commit only).
