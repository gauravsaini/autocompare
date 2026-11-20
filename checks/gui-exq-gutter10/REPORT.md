# REPORT: GUTTER10 numbers plus overview (BC parity)

Round: checks/gui-exq-gutter10/round-1/ -> RESULT:PASS (round-1 & round-2 complete)
Harness: PNG 10401 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: gutter consts/helpers reuse, SPACE tokens LG/MD/SM/XL/XS sab reuse,
  TypoRole x807, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, gutter6 baseline se zero growth)

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Line-number text origin | column width to thi (`gutter_linenum_column_width`), par text kahan se shuru ho — koi headless contract nahi (paint ad-hoc) | `GUTTER_LINENUM_INSET` = GUTTER_MERGE_GAP reuse (2px) + `gutter_linenum_text_x(col_x, col_w)` = col_x + col_w - inset (0,16 -> 14.0); patli/NaN column par None |
| Padded minimap click-mapping | `gutter_row_at_y` poore `[0,height]` par napta hai, par padded bands `[pad,height-pad]` me hain — kinaaron par click galat row par girta | `gutter_padded_row_at_y`: fraction inner track se, pad kinaare nearest row par clamp (y=2 -> 0, y=599 -> 5), ruler-bahar None |
| Padded markers | selection marker ke liye padded inverse nahi tha | `gutter_padded_y_for_row`: pad + (row/total)*inner (row0 -> 4.0, row5 -> 497.333); marker y wapas usi row par round-trip |
| Viewport band padded track me | `gutter_viewport_rect` full-height napta hai, padded paint me band kinare clip karta | `gutter_viewport_rect_padded`: wahi window-mapping `[pad,pad+inner]` me + 4px floor andar (full -> (4,596), tiny 1000-row -> (200,204)); round-2 bottom-edge floor lock ((392,396) at row 999) |
| Narrow 720 side marker | 960 layout tha (`gutter_side_layout_960`), par narrow 720 par side gutter marker alignment contract nahi tha | `gutter_side_layout_720` (12, 338, 20, 338, 12) + `gutter_side_column_x_720` (350.0) + `gutter_side_arrow_cx_720` (360.0, exact canvas middle) + `gutter_side_glyph_720` (354..366) + `gutter_side_hit_band_720` (348..372) |
| Narrow 720 center gutter | 720 center layout tha (`gutter_text_layout_720`), par center arrow cx helper nahi tha | `gutter_center_column_x_720` (348.0) + `gutter_center_arrow_cx_720` (360.0) + `gutter_center_glyph_720` (354..366) — side aur center markers dono canvas mid (360) par align |
| Char metrics at 720 | Number column char-metric scaling tha, par 720 pane me marker alignment contract verify nahi tha | Left pane col_x 12 + col_w 16 - inset 2 = 26.0 text_x; right pane 370 + 16 - 2 = 384.0 text_x; text gutter 32px pane 338px ke andar safe |
| Marker visibility | Gutter-side markers Equal rows par chhupe hone chahiye | `gutter_side_marker_visible(Equal) == false`, diff kinds (`Deleted`, `Inserted`, `Changed`) == true; exactly 3 changed rows [1, 3, 5] |

Typography second: koi naya role/size nahi — numbers wahi `content_grid` (Mono 12 tabular) me; GUTTER10 me zero `.size(` call.
Colors: FROZEN, gutter.rs me zero Color32/Visuals (repo-wide 33, gutter6 baseline se zero growth).

## Exact lines changed

crates/gui/src/gutter.rs:
- GUTTER10 section: 1 const (LINENUM_INSET=GUTTER_MERGE_GAP) + 12 fns (linenum_text_x, padded_row_at_y, padded_y_for_row, viewport_rect_padded, side_layout_720, side_column_x_720, side_arrow_cx_720, side_glyph_720, side_hit_band_720, center_column_x_720, center_arrow_cx_720, center_glyph_720).
- exq_gutter10_tests: 4 tests, fixtures:
  - `checks/gui-exq-gutter10/round-1/comparison/edge-padded-click-viewport.json` mirror (6-row E D D E E I @600px: text_x 14.0/118.0, pad-clamp 0/5, round-trip, viewport (102.67,300)/(4,596)/(200,204)/(392,396)).
  - `checks/gui-exq-gutter10/round-1/comparison/gutter-narrow-720.json` mirror (8-row mixed @800px: side layout (12,338,20,338,12), col_x 350, cx 360, glyph 354..366, hit band 348..372; center layout col_x 348, cx 360; char metrics linenum col 16 -> text_x 26/384, text gutter 32 < 338 pane; changed rows [1,3,5], 3 segments).
- fmt: rustfmt clean; clippy lib+tests zero warning (match->`?`, float precision fix samet).

## Gates

cargo test -p autocompare-gui 1004 passed / 0 failed; clippy -D warnings clean; rustfmt --edition 2024 --check clean;
banned non-test (unwrap/expect/panic/todo/unreachable/indexing) 0; harness RESULT:PASS.
Wiring note: paint adoption views.rs owner ke paas (number text origin, padded click/marker/viewport, 720 narrow alignment) — yahan sirf headless contract + tests.
