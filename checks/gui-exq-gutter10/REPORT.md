# REPORT: GUTTER10 numbers plus overview (BC parity)

Round: checks/gui-exq-gutter10/round-1/ -> RESULT:PASS (1 round, repeat nahi chahiye tha)
Harness: PNG 10401 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: gutter x1036 (pehle 898, +138 naye headless contract se), SPACE tokens LG/MD/SM/XL/XS sab reuse,
  TypoRole x807, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, gutter6 baseline se zero growth)

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Line-number text origin | column width to thi (`gutter_linenum_column_width`), par text kahan se shuru ho — koi headless contract nahi (paint ad-hoc) | `GUTTER_LINENUM_INSET` = GUTTER_MERGE_GAP reuse (2px) + `gutter_linenum_text_x(col_x, col_w)` = col_x + col_w - inset (0,16 -> 14.0); patli/NaN column par None |
| Padded minimap click-mapping | `gutter_row_at_y` poore `[0,height]` par napta hai, par padded bands `[pad,height-pad]` me hain — kinaaron par click galat row par girta | `gutter_padded_row_at_y`: fraction inner track se, pad kinaare nearest row par clamp (y=2 -> 0, y=599 -> 5), ruler-bahar None |
| Padded markers | selection marker ke liye padded inverse nahi tha | `gutter_padded_y_for_row`: pad + (row/total)*inner (row0 -> 4.0, row5 -> 497.333); marker y wapas usi row par round-trip |
| Viewport band padded track me | `gutter_viewport_rect` full-height napta hai, padded paint me band kinare clip karta | `gutter_viewport_rect_padded`: wahi window-mapping `[pad,pad+inner]` me + 4px floor andar (full -> (4,596), tiny 1000-row -> (200,204)) |

Typography second: koi naya role/size nahi — numbers wahi `content_grid` (Mono 12 tabular) me; GUTTER10 me zero `.size(` call.
Colors: FROZEN, gutter.rs me zero Color32/Visuals (repo-wide 33, gutter6 se unchanged).

## Exact lines changed

crates/gui/src/gutter.rs (209 insert, 0 delete — pure append):
- GUTTER10 section: 1 const (LINENUM_INSET=GUTTER_MERGE_GAP) + 4 fns (linenum_text_x, padded_row_at_y, padded_y_for_row, viewport_rect_padded)
- exq_gutter10_tests: 3 tests, fixture checks/gui-exq-gutter10/round-1/comparison/edge-padded-click-viewport.json mirror (6-row E D D E E I @600px: text_x 14.0/118.0, pad-clamp 0/5, round-trip, viewport (102.67,300)/(4,596)/(200,204))
- fmt: rustfmt clean; clippy lib+tests zero warning (match->`?`, float precision fix samet)

## Gates

cargo test -p autocompare-gui 870 passed / 0 failed; clippy -D warnings clean; rustfmt --edition 2024 --check clean;
banned non-test (unwrap/expect/panic/todo/unimplemented) 0; harness RESULT:PASS.
Wiring note: paint adoption views.rs owner ke paas (number text origin, padded click/marker/viewport) — yahan sirf headless contract + tests.
