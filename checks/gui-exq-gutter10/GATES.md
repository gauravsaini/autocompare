# GATES: GUTTER10 numbers plus overview (BC parity)

OWNS: crates/gui/src/gutter.rs, checks/gui-exq-gutter10/
NO-TOUCH: app.rs, theme.rs, views.rs, views_hex.rs, views_table.rs, copy.rs, take.rs, tree.rs, screenshot.rs, viewmodel.rs, STATUS.md, PLAN.md, inbox/build-progress.json

Scope: line-number text origin (right-aligned, fixed inset) + padded
minimap click-mapping (row<->y, viewport band inner track me). Geometry
FIRST (sab SPACE/GUTTER scale reuse), typography SECOND (existing
content_grid Mono 12 tabular reuse, koi naya size nahi). Colors FROZEN
(zero Color32/Visuals touch).

## Observable gates (sab CHECK/EXPECT/EVIDENCE ke saath)

- [x] G1: number text origin right-aligned, fixed inset, scale par
  CHECK: `cargo test -p autocompare-gui gutter10_text_x` green + `grep -n "gutter_linenum_text_x" crates/gui/src/gutter.rs`
  EXPECT: INSET == GUTTER_MERGE_GAP == 2.0; (0,16)->14.0; (100,20)->118.0; col_w<=inset/NaN par None
  EVIDENCE: round-1 green (870 tests ok, harness RESULT:PASS, PNG 10401B distinct 256)

- [x] G2: padded row_at_y pad-kinaron par nearest row par gire, bahar None
  CHECK: `cargo test -p autocompare-gui gutter10_padded_row` green + fixture comparison/edge-padded-click-viewport.json
  EXPECT: 6 rows @600px (pad 4, inner 592): y=4.0->0, y=2.0->0, y=599.0->5, y=600.0->None, y<0->None
  EVIDENCE: round-1 green (870 tests ok, harness RESULT:PASS)

- [x] G3: padded y_for_row inverse, viewport band inner track me floor samet
  CHECK: `cargo test -p autocompare-gui gutter10_padded_markers` green
  EXPECT: row0->4.0, row5->497.33; viewport (6,600,1,2)->(102.67,300.0), full->(4,596), tiny (1000,400,500,1)->(200,204)
  EVIDENCE: round-1 green (870 tests ok, harness RESULT:PASS)

- [x] G4: typo reuse, koi naya size nahi
  CHECK: `grep -n "content_grid" crates/gui/src/gutter.rs` + exquisite-report me koi naya `.size` nahi
  EXPECT: numbers wahi content_grid (Mono 12 tabular); GUTTER10 me zero `.size(` call
  EVIDENCE: round-1 green; report TypoRole x807, .size ['14.0'] only, GUTTER10 me 0 naye size

- [x] G5: MS Rust + frozen gates
  CHECK: `cargo clippy -p autocompare-gui -- -D warnings` + `rustfmt --edition 2024 --check crates/gui/src/gutter.rs` + banned grep + `grep -c Color32 gutter.rs` == 0
  EXPECT: clippy 0 warnings, fmt clean, non-test unwrap/expect/panic/todo/unimplemented 0, colors 0
  EVIDENCE: round-1 green; clippy lib+tests clean, fmt clean, banned 0, gutter.rs colors 0 (repo 33, gutter6 se unchanged)

## Round-2 Observable gates (markers plus narrow-720 viewport lock)

- [x] G6: narrow 720px side marker layout + alignment at canvas middle (cx=360) from char metrics
  CHECK: `cargo test -p autocompare-gui gutter10_narrow_720` green + fixture comparison/gutter-narrow-720.json
  EXPECT: layout (12, 338, 20, 338, 12), col_x 350, cx 360 (canvas mid), glyph (354, 194, 366, 206), hit band (348, 372) symmetric about 360; char metrics linenum col 16 -> text_x 26.0 (col 12 + 16 - 2)
  EVIDENCE: round-2 green (1004 tests ok, harness RESULT:PASS)

- [x] G7: center column 24px at 720 also centers at cx=360 (col_x 348, cx 360)
  CHECK: `cargo test -p autocompare-gui gutter10_narrow_720` green
  EXPECT: center col_x 348, cx 360, glyph centered at cx=360
  EVIDENCE: round-2 green; center column 348 + 12 = 360, glyph matches side marker center

- [x] G8: marker visibility only on changed rows
  CHECK: `cargo test -p autocompare-gui gutter10_narrow_720` green
  EXPECT: Equal -> false, Deleted/Inserted/Changed -> true; changed_rows -> [1, 3, 5] on 8-row mixed
  EVIDENCE: round-2 green; Equal rows blank, diff kinds get markers, exactly 3 segments

- [x] G9: padded click viewport bottom edge floor lock
  CHECK: `cargo test -p autocompare-gui gutter10_padded` green
  EXPECT: 1000 rows @400px bottom row 999 keeps floor >= 4.0 within [pad, pad + inner] -> (392, 396)
  EVIDENCE: round-2 green; bottom row viewport band never collapses below GUTTER_VIEWPORT_MIN_H

- [x] G10: MS Rust + frozen gates round-2
  CHECK: `cargo clippy -p autocompare-gui -- -D warnings` + `rustfmt --edition 2024 --check crates/gui/src/gutter.rs` + banned grep + `grep -c Color32 gutter.rs` == 0
  EXPECT: clippy 0 warnings, fmt clean, non-test unwrap/expect/panic/todo/unreachable/indexing 0, colors 0
  EVIDENCE: round-2 green; clippy clean, fmt clean, banned 0, gutter.rs colors 0 (repo 33, frozen)
