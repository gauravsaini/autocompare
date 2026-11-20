# REPORT: GUTTER6 numbers column plus minimap overview (BC parity)

Round: checks/gui-exq-gutter6/round-1/ -> RESULT:PASS (1 round, repeat nahi chahiye tha)
Harness: PNG 10401 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: gutter x898 (pehle 749, +149 naye headless contract se), SPACE tokens LG/MD/SM/XL/XS sab reuse,
  TypoRole x734, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, zero growth)

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Line-number column width | fixed ruler 16/8, digit count se independent (4-digit numbers crush) | `gutter_linenum_column_width(max_no, show)`: 16 + har extra digit par 4 (16/20/24/28/32, cap 36), chhupa par 8; sab 4-scale par |
| Line-number digits | koi headless contract nahi | `gutter_linenum_digits`: decimal digits, 0 par bhi 1; cap `GUTTER_LINENUM_MAX_EXTRA` = 5 |
| Text-row padding vs rows | pad ad-hoc (views ROW_PAD = 2 alag token se) | `GUTTER_TEXT_ROW_PAD` = GUTTER_MERGE_GAP reuse (2px/side) + `gutter_text_row_height(line_h)` = line + 2*pad (15.6 -> 19.6); kharab input par None |
| Thumbnail overview same/diff heights | sirf diff segments (`gutter_geometry` Equal skip karta hai) | `gutter_overview_geometry`: Equal runs background bands bante hain, wahi fraction-mapping + 4px floor + 2px fusion; diff bands dono paths par byte-identical |
| Arrow hit-targets | 16px box hi click-target (4px ruler bands ungli se nahi lagte) | `GUTTER_ARROW_HIT_SLOP` = 2px reuse + `gutter_arrow_hit_rect` (16px box -> 20px target) + `gutter_arrow_hit_contains` (edges inclusive, NaN par false) |

Typography second: `gutter_linenum_role` = content_grid (Mono 12 tabular, existing alias), `gutter_linenum_is_tabular` = true.
Koi naya font size, weight, letter-spacing nahi. Colors: FROZEN, gutter.rs me zero Color32/Visuals.

## Exact lines changed

crates/gui/src/gutter.rs (337 insert, 0 delete — pure append):
- GUTTER6 section (~1337-1520): 3 consts (LINENUM_PAD=SPACE_XS, TEXT_ROW_PAD=GUTTER_MERGE_GAP, ARROW_HIT_SLOP=GUTTER_MERGE_GAP, LINENUM_MAX_EXTRA=5) + 10 fns (digits, column_width, linenum_role, linenum_is_tabular, text_row_pad, text_row_height, overview_geometry, arrow_hit_rect, arrow_hit_contains)
- exq_gutter6_tests (~1525+): 5 tests, fixture checks/gui-exq-gutter6/round-1/comparison/edge-tall-same-diff.json mirror (6-row E D D E E I @600px: 4 overview segments, diff-band parity, 16->20 hit rect)
- fmt: rustfmt clean; clippy lib+tests me gutter.rs zero error/warning (views_table.rs:3110 test-only useless_vec pre-existing hai, forbidden file isliye untouched)

## Gates

cargo test -p autocompare-gui 840 passed / 0 failed; clippy -D warnings lib clean; rustfmt --edition 2024 --check clean;
banned non-test (unwrap/expect/panic/todo/unimplemented) 0; harness RESULT:PASS.
Wiring note: paint adoption views.rs owner ke paas (numbers column width, overview same+diff bands, arrow hit rect) — yahan sirf headless contract + tests.
