# REPORT: GUTTERCOPY8 arrows plus minimap (BC parity)

Round: checks/gui-exq-guttercopy8/round-1/ -> RESULT:PASS (round-1/6, repeat nahi chahiye tha)
Harness: PNG 10401 bytes (min 8192 ok), magic ok, distinct 256 (ok)
Signals: gutter x969 (pehle 898, +71 naye headless contract se), SPACE tokens LG/MD/SM/XL/XS sab reuse,
  TypoRole x755, .size ['14.0'] (koi naya size nahi), colors 33 (frozen, zero growth)

## Geometry table before -> after

| Area | Before | After |
|---|---|---|
| Arrow chevron vs box | 16px box hi visual tha (chevron kinare chhoota tha) | `GUTTER_ARROW_GLYPH_INSET` = 2px reuse + `gutter_arrow_glyph_rect(cx, cy)`: 16px box me 12px chevron (6..18, 194..206); 20px slop hit untouched |
| Arrow x centering | paint ko column me center ka headless contract nahi | `gutter_arrow_center_x(col_x)` = col_x + 24/2 (col 0 -> 12); NaN/Inf par None |
| Minimap track edges | overview bands [0, height] ruler kinare chipakte the (pehli/aakhri clip) | `GUTTER_MINIMAP_TRACK_PAD` = SPACE_XS (4) + `gutter_minimap_inner_height` (400 -> 392; 8/0/neg/NaN par None) + `gutter_overview_geometry_padded`: bands [4, 596] me, contiguous, same+diff shared band-math |
| Copy visual-vs-hit | sirf 24px hit tha, chevron size ka contract nahi | `COPY_HUNK_ARROW_GAP` = tight gap (4) + `copy_arrow_visual_size` = 24 - 2*4 = 16 (gutter box parity) + `copy_hunk_arrow_pitch(row_h)` = row + gap (19.6 -> 23.6) taaki stacked arrows collide na karen |
| Take conflicts on minimap | conflict rows ka ruler-band mapping nahi tha | `TAKE_MINIMAP_MARK_MIN_H` = XS (4, overview floor parity) + `take_minimap_band` (row 500/1000 @400 -> 200..204) + `take_minimap_marks` (order me, kharab rows skip) |

Typography second: koi text change nahi (koi naya size/weight/spacing nahi). Colors: FROZEN, owned files me zero Color32/Visuals.

## Exact lines changed

crates/gui/src/gutter.rs (append-only GUTTERCOPY8 section + exq_guttercopy8_tests, 5 tests):
- 2 consts (GLYPH_INSET=GUTTER_MERGE_GAP, MINIMAP_TRACK_PAD=SPACE_XS) + 4 fns (glyph_rect, center_x, inner_height, overview_padded)
- fixture checks/gui-exq-guttercopy8/round-1/comparison/edge-padded-minimap-tall.json mirror (1000-row tall, 400px ruler, pad 4 -> inner 392; glyph 6..18/194..206; padded 4 segments [4,596]; tall diff band 4px floor)

crates/gui/src/copy.rs (append-only, 1 test):
- 1 const (HUNK_ARROW_GAP=GEO_TIGHT_GAP) + 2 fns (visual_size == gutter box 16, pitch; bad input None)

crates/gui/src/take.rs (append-only, 2 tests):
- 1 const (MINIMAP_MARK_MIN_H=SPACE_XS == gutter floor) + 2 fns (band, marks)

## Gates

cargo test -p autocompare-gui 858 passed / 0 failed (8 naye); clippy --all-targets -D warnings clean;
rustfmt --edition 2024 --check clean; banned non-test (unwrap/expect/panic/todo/unimplemented) 0;
owned Color32/Visuals 0; harness RESULT:PASS.
Note: beech me ek foreign hexdense8 WIP test (views_hex.rs, forbidden file) RED tha — meri files se unrelated,
ussi wave ke fix se green hua; final tree full green.
Wiring note: paint adoption views.rs owner ke paas (glyph rect chevron, padded overview bands, arrow center-x, copy pitch, take marks) — yahan sirf headless contract + tests.
