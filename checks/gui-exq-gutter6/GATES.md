# GATES: GUTTER6 numbers column plus minimap overview (BC parity)

OWNS: crates/gui/src/gutter.rs, checks/gui-exq-gutter6/
NO-TOUCH: app.rs, theme.rs, views.rs, views_hex.rs, views_table.rs, copy.rs, take.rs, tree.rs, screenshot.rs, viewmodel.rs, STATUS.md, PLAN.md, inbox/build-progress.json

Scope: gutter line-number column width (digit scaling), text-row pad vs rows, thumbnail overview same/diff band parity,
center-arrow slop hit-targets. Geometry FIRST (sab SPACE scale reuse), typography SECOND (existing Mono 12 tabular reuse).
Colors FROZEN (zero Color32/Visuals touch).

## Observable gates (sab CHECK/EXPECT/EVIDENCE ke saath)

- [x] G1: line-number column digit scaling, scale par
  CHECK: `cargo test -p autocompare-gui gutter6_numbers` green + `grep -n "gutter_linenum_column_width" crates/gui/src/gutter.rs`
  EXPECT: 1-digit 16.0, 3-digit 24.0, 7-digit cap 36.0, hidden 8.0; PAD == SPACE_XS
  EVIDENCE: round-1 green (840 tests ok, harness RESULT:PASS, PNG 10401B distinct 256)

- [x] G2: numbers typo grid-mono-tabular reuse, koi naya size nahi
  CHECK: `grep -n "gutter_linenum_role\|gutter_linenum_is_tabular" crates/gui/src/gutter.rs` + exquisite-report me `.size ['14.0']` only
  EXPECT: role == TypoRole::content_grid() == Mono 12.0, tabular true
  EVIDENCE: round-1 green (840 tests ok, harness RESULT:PASS)

- [x] G3: text-row pad vs line height ek beat par
  CHECK: `cargo test -p autocompare-gui gutter6_text_row` green
  EXPECT: pad 2.0 == GUTTER_MERGE_GAP; 15.6 -> 19.6; 0/neg/NaN/Inf par None
  EVIDENCE: round-1 green (840 tests ok, harness RESULT:PASS)

- [x] G4: overview same+diff bands, diff-band parity
  CHECK: `cargo test -p autocompare-gui gutter6_overview` green + fixture comparison/edge-tall-same-diff.json
  EXPECT: E D D E E I @600px -> 4 segments (Equal/Deleted/Equal/Inserted), diff bands diff-only geometry se identical, contiguous 0..600
  EVIDENCE: round-1 green (840 tests ok, harness RESULT:PASS)

- [x] G5: arrow 16px box, 20px slop hit-target, paint unmoved
  CHECK: `cargo test -p autocompare-gui gutter6_arrow` green
  EXPECT: hit rect (2.0,190.0,22.0,210.0); box-miss par hit lage (x=3.0), bahar false, NaN par None/false
  EVIDENCE: round-1 green (840 tests ok, harness RESULT:PASS)

- [x] G6: MS Rust + frozen gates
  CHECK: `cargo clippy -p autocompare-gui -- -D warnings` + `rustfmt --edition 2024 --check crates/gui/src/gutter.rs` + banned grep + `grep -c Color32 gutter.rs` == 0
  EXPECT: clippy 0 warnings, fmt clean, non-test unwrap/expect/panic/todo/unimplemented 0, colors 0
  EVIDENCE: round-1 green; views_table.rs:3110 test-only useless_vec pre-existing hai, forbidden file isliye untouched (lib clippy clean)
