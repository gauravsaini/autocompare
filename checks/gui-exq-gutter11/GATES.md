# GATES: GUTTER11 tall plus center (BC parity round-2)

OWNS: crates/gui/src/gutter.rs, checks/gui-exq-gutter11/
NO-TOUCH: app.rs, theme.rs, views.rs, views_hex.rs, views_table.rs, copy.rs, take.rs, tree.rs, screenshot.rs, viewmodel.rs, STATUS.md, PLAN.md, inbox/build-progress.json

Scope: Exquisite gutter parity round-2. Tall-1200 center column (1200px width/height, symmetric layout, cx=600 or center col 20px / 24px), marker x lock over long scroll, click viewport floor tall. Geometry FIRST, typography SECOND, colors FROZEN at 33 touches.

## Observable gates (sab CHECK/EXPECT/EVIDENCE ke saath)

- [x] G1: tall-1200 side marker layout + alignment at canvas middle (cx=600)
  CHECK: `cargo test -p autocompare-gui gutter11_tall_1200` green + fixture comparison/gutter-tall-1200.json
  EXPECT: layout (12, 578, 20, 578, 12), col_x 590, cx 600 (canvas mid), glyph (594, 594, 606, 606), hit band (588, 612) symmetric about 600
  EVIDENCE: round-1 green (1027 tests ok, harness RESULT:PASS, PNG 10401B distinct 256)

- [x] G2: center column 24px at 1200 also centers at cx=600 (col_x 588, cx 600)
  CHECK: `cargo test -p autocompare-gui gutter11_tall_1200` green
  EXPECT: center col_x 588, cx 600, center layout (12, 576, 24, 576, 12), glyph centered at cx=600
  EVIDENCE: round-1 green; center column 588 + 12 = 600, glyph matches side marker center

- [x] G3: marker tall lock over long scroll
  CHECK: `cargo test -p autocompare-gui gutter11_tall_1200` green
  EXPECT: markers stay locked at horizontal center cx=600 across all valid row indices (0..1200) during tall scroll
  EVIDENCE: round-1 green; cx=600 verified for rows 0, 10, 100, 500, 600, 1100, 1199

- [x] G4: viewport tall floor over 1200 scroll
  CHECK: `cargo test -p autocompare-gui gutter11_tall_1200` green
  EXPECT: viewport band never collapses below GUTTER_VIEWPORT_MIN_H (4.0) over tall 1200 scroll; mid viewport (598, 602) and bottom viewport (1192, 1196) remain strictly within inner track [4, 1196]
  EVIDENCE: round-1 green; floor >= 4.0 verified, row 1199 yields (1192, 1196) without escaping track

- [x] G5: marker visibility only on changed rows
  CHECK: `cargo test -p autocompare-gui gutter11_tall_1200` green
  EXPECT: Equal -> false, Deleted/Inserted/Changed -> true; changed_rows -> [1, 3, 5] on 8-row mixed
  EVIDENCE: round-1 green; Equal rows blank, diff kinds get markers, exactly 3 segments

- [x] G6: typo reuse, zero new font sizes
  CHECK: `grep -n "content_grid" crates/gui/src/gutter.rs` + exquisite-report me koi naya `.size` nahi
  EXPECT: numbers wahi content_grid (Mono 12 tabular); GUTTER11 me zero `.size(` call
  EVIDENCE: round-1 green; report TypoRole x1048, .size ['14.0'] only, GUTTER11 me 0 naye size

- [x] G7: MS Rust compliance
  CHECK: `cargo clippy -p autocompare-gui -- -D warnings` + `rustfmt --edition 2024 --check crates/gui/src/gutter.rs` + banned grep
  EXPECT: clippy 0 warnings, fmt clean, non-test unwrap/expect/panic/todo/unreachable/indexing 0
  EVIDENCE: round-1 green; clippy clean, fmt clean, banned 0

- [x] G8: colors frozen check
  CHECK: `grep -c Color32 crates/gui/src/gutter.rs` == 0
  EXPECT: colors 0 (repo-wide 33 touches frozen)
  EVIDENCE: round-1 green; gutter.rs colors 0 (repo 33, frozen)
