# GATES — Wave AX-PAINTWIRE (checks/gui-paintwire-ax)

Live paint me wired contracts adopt karo (AW2 feedback `checks/localrun-aw2/REPORT.md` §9.1/§9.2):
text overview-ruler paint width gutter contract par + `paint_home` home contract par.
Geometry first, typography second (same sizes/weights, koi naya size nahi). Colors FROZEN.

## OWNS / KEVL

- OWNS: `crates/gui/src/views.rs` + `checks/gui-paintwire-ax/**` ONLY.
- KEVL (haath mana — read-only samajh ke liye ok): `screenshot.rs`, `gutter.rs`,
  `home.rs`, `app.rs`, `theme.rs`, `viewmodel.rs`, `views_table.rs`, `views_hex.rs`,
  `checks/localrun-aw2/**`, `checks/opencode-waves/STATUS.md` (orchestrator-owned).
- Contracts ko sirf read kiya; parallel waves ke files me zero edit.

## Design (geometry first, typography second, colors frozen)

- Ruler: `text_minimap_ruler_width` 12/6 → `gutter_ruler_width_for_font` 16/8
  (legacy `text_gutter_paint_width` 12/6 untouched — `GUTTER_W` chrome lock barkarar).
  `SEGMENT_MIN_H` 4.0 paint geometry me `viewmodel::gutter_geometry` (gutter
  contract re-export) se aata hai; slop hit-test `text_ruler_hit_slop` = 2.0
  (`GUTTER_HIT_SLOP`) + `text_ruler_segment_hit`, paint click gate me wired.
  Dono directions: `gutter_y_for_row` (band top, marker) + naya
  `text_minimap_row_center_y` (`gutter_row_center_y`, round-trip tested).
- Home (`paint_home`): card min-h `home_paint_card_height` = 99.6 (legacy 75 floor),
  CTA `add_sized` 120x28 (`home_paint_cta_size`, hit 3360 vs purana 2640),
  grid `home_paint_grid_columns` = 3 + `HOME_GRID_GAP` 8 rhythm, title strong-14
  vs desc small-weak-11 hierarchy (same sizes/weights), Home par path picker
  nahi (`debug_assert!(!HOME_SHOWS_PATH_PICKER)`, bachta chrome 32).
  Recent columns: naya `paint_home_recent` (title flex + kind 96 + age 72 via
  `recent_row_columns`, do-line rows, honest empty hint) + pure
  `home_recent_row_title`; App wiring (history + now_secs) caller wave ka hai.
- Colors: zero `Color32`/`Visuals`/palette touch (frozen).

## Gates (sab PASS chahiye)

1. `cargo test -p autocompare-gui` — full suite green (2 naye tests samet).
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — zero warnings.
3. `cargo fmt -- --check crates/gui/src/views.rs` — clean (sirf OWNS file).
4. Banned grep OWNS non-test me: `.unwrap()`/`.expect(`/`panic!`/`todo!`/
   `unimplemented!`/`unreachable!` — zero hits; `#![forbid(unsafe_code)]` kept.
5. `tools/gui-polish/run-geo-typo.sh checks/gui-paintwire-ax/round-1` —
   `RESULT: PASS` (PNG magic ok, >= 8192B, non-blank).
6. `REPORT.md` + `round-1/` artifacts, phir `git add` sirf OWNS paths +
   `git commit` (conventional English). No push, no stash.

## Banned (MS rust guidelines)

- Non-test code me `unwrap/expect/panic/todo` mana; `forbid(unsafe_code)` kept.
- Repo ke bahar copy/sandbox nahi; dusre agent ki files mat chhoona.
- `git stash/push` kabhi nahi; `STATUS.md` ko haath nahi.
