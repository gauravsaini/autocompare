# GATES: GUTTERCOPY8 arrows plus minimap (BC parity)

OWNS: crates/gui/src/gutter.rs, crates/gui/src/copy.rs, crates/gui/src/take.rs, checks/gui-exq-guttercopy8/
NO-TOUCH: app.rs, theme.rs, views.rs, views_hex.rs, views_table.rs, views_table.rs, tree.rs, home.rs, viewmodel.rs, screenshot.rs, STATUS.md, PLAN.md, inbox/build-progress.json

Scope: center-arrow chevron breathing (glyph vs box vs hit), arrow column centering,
padded minimap track (overview bands ruler edges se andar), copy visual-vs-hit + hunk pitch,
take conflict minimap bands. Geometry FIRST (sab SPACE scale reuse), typography SECOND (koi naya size nahi).
Colors FROZEN (zero Color32/Visuals touch).

## Observable gates (sab CHECK/EXPECT/EVIDENCE ke saath)

- [x] G1: arrow glyph box ke andar saans leta hai, hit untouched
  CHECK: `cargo test -p autocompare-gui --lib -- guttercopy8` me `glyph_box_breathes_inside_hit_box` green
  EXPECT: glyph (6.0,194.0,18.0,206.0); hit (3.0,200.0) lage; NaN/Inf par None; INSET == GUTTER_MERGE_GAP == 2.0
  EVIDENCE: round-1 green (858 tests ok, harness RESULT:PASS, PNG 10401B distinct 256)

- [x] G2: arrow 24px column me centered
  CHECK: same run me `arrow_centers_in_column` green
  EXPECT: center_x(0.0) == 12.0, center_x(100.0) == 112.0; NaN/Inf par None
  EVIDENCE: round-1 green

- [x] G3: padded minimap track, bands edges se andar
  CHECK: `minimap_track_pads_tiny_heights_out` + `padded_overview_sits_inside_track` + `tall_file_bands_stay_inside_track` green + fixture comparison/edge-padded-minimap-tall.json
  EXPECT: pad 4 == SPACE_XS; inner(400) == 392; inner(8)/0/neg/NaN None; 6-row E D D E E I @600 -> 4 segments [4,596] contiguous; tall diff band 4px floor, sab [4,396] ke andar
  EVIDENCE: round-1 green

- [x] G4: copy visual-vs-hit + hunk pitch rhythm
  CHECK: `copy::exq_guttercopy8_tests` green
  EXPECT: GAP == GEO_TIGHT_GAP == 4.0; visual 16.0 == GUTTER_ARROW_BOX < hit 24.0; pitch(19.6) == 23.6; bad par None
  EVIDENCE: round-1 green

- [x] G5: take conflict minimap bands
  CHECK: `take::exq_guttercopy8_tests` green
  EXPECT: MARK_MIN_H == SPACE_XS == GUTTER_SEGMENT_MIN_H == 4.0; band(1,4,400) == (100,200); band(500,1000,400) == (200,204); marks order me, OOB skip; OOB/0/NaN par None/khali
  EVIDENCE: round-1 green

- [x] G6: MS Rust + frozen gates
  CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` + `rustfmt --edition 2024 --check` owned files + banned grep + `grep -c Color32/Visuals` owned == 0
  EXPECT: clippy 0 warnings, fmt clean, non-test unwrap/expect/panic/todo/unimplemented 0, colors 0
  EVIDENCE: round-1 green; foreign hexdense8 WIP RED beech me aaya tha (forbidden views_hex.rs), ussi wave ke fix se green — mere owned diff se unrelated
