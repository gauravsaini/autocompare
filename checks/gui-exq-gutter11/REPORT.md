# REPORT: GUTTER11 tall plus center (BC parity round-2)

Round: checks/gui-exq-gutter11/round-1/ -> RESULT:PASS
Harness: PNG 10401 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: gutter consts/helpers reuse, SPACE tokens LG/MD/SM/XL/XS sab reuse,
  TypoRole x1048, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, gutter.rs me zero Color32/Visuals touches)

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Tall-1200 side column | 960/720 layouts the (`gutter_side_layout_960`, `gutter_side_layout_720`), par 1200 tall canvas par symmetric side gutter layout contract nahi tha | `gutter_side_layout_1200` (12, 578, 20, 578, 12) + `gutter_side_column_x_1200` (590.0) + `gutter_side_arrow_cx_1200` (600.0, exact canvas middle) + `gutter_side_glyph_1200` (594..606 x 594..606) + `gutter_side_hit_band_1200` (588..612 symmetric about 600) |
| Tall-1200 center column | 1200 canvas par 24px center divider layout contract loose tha | `gutter_text_layout_1200` (12, 576, 24, 576, 12) + `gutter_center_column_x_1200` (588.0) + `gutter_center_arrow_cx_1200` (600.0, canvas middle) + `gutter_center_glyph_1200` (594..606 x 594..606) — side aur center dono cx=600 par lock |
| Marker tall lock over long scroll | Long scroll par markers ka horizontal alignment drift kar sakta tha | `gutter_marker_tall_lock_cx_1200` aur `gutter_center_marker_tall_lock_cx_1200`: har valid row index (0..1200) par cx strictly locked at 600.0, koi horizontal drift nahi |
| Viewport tall floor over 1200 scroll | Tall 1200 scroll par tiny window viewport band collapse ya track clip hone ka risk tha | `gutter_viewport_tall_floor_1200`: guaranteed min-h 4.0 floor (`GUTTER_VIEWPORT_MIN_H`), mid viewport (598, 602) aur bottom viewport (1192, 1196) inner track [4, 1196] ke andar locked |
| Reused GUTTER10 gates | Narrow-720 layout, char-metrics text origin, padded click viewport, marker visibility only on changed rows | Sab reused and verified green |

Typography second: koi naya role/size nahi — numbers wahi `content_grid` (Mono 12 tabular) me; GUTTER11 me zero `.size(` call.
Colors: FROZEN, gutter.rs me zero Color32/Visuals (repo-wide 33, frozen baseline se zero growth).

## Exact lines changed

crates/gui/src/gutter.rs:
- GUTTER11 section: 12 fns (side_layout_1200, side_column_x_1200, side_arrow_cx_1200, side_glyph_1200, side_hit_band_1200, text_layout_1200, center_column_x_1200, center_arrow_cx_1200, center_glyph_1200, marker_tall_lock_cx_1200, center_marker_tall_lock_cx_1200, viewport_tall_floor_1200).
- exq_gutter11_tests: test `gutter11_tall_1200_center_column_and_marker_lock` mirroring `checks/gui-exq-gutter11/round-1/comparison/gutter-tall-1200.json`.
- fmt: rustfmt clean; clippy lib+tests zero warning.

## Gates

cargo test -p autocompare-gui 1027 passed / 0 failed; clippy -D warnings clean; rustfmt --edition 2024 --check clean;
banned non-test (unwrap/expect/panic/todo/unreachable/indexing) 0; harness RESULT:PASS.
Wiring note: paint adoption views.rs owner ke paas — yahan sirf headless contract + tests.
