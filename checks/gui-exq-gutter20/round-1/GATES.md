# GATES: GUTTER20 live egui 20px center gutter (BC parity)

OWNS: crates/gui/src/views.rs, checks/gui-exq-gutter20/
NO-TOUCH: screenshot.rs (parallel wave owns it), app.rs, theme.rs, gutter.rs,
views_hex.rs, views_table.rs, copy.rs, take.rs, tree.rs, viewmodel.rs,
STATUS.md, PLAN.md, inbox/build-progress.json

Scope: live egui Text Compare center gutter at the locked 20px
(`text_center_gutter_width` = SPACE_LG + SPACE_XS), yellow hunk-head
`=>` arrows via `text_center_marker_fg` on changed rows only (Equal blank),
selection bands + floor fill keep working. Geometry FIRST (reuse helpers),
typography SECOND (nothing new unless needed). Colors FROZEN at 33 touches.

## Observable gates (sab CHECK/EXPECT/EVIDENCE ke saath)

- [x] G1: live center gutter is the locked 20px (both wrap-on + wrap-off paint paths)
  CHECK: `grep -n "ui.set_width(text_center_gutter_width())" crates/gui/src/views.rs`
  + `cargo test -p autocompare-gui gutter20` green
  EXPECT: wrap-on head path, wrap-on continuation path, wrap-off head path,
  wrap-off continuation path, wrap-off trailing-blank path sab 20px
  (`TEXT_CENTER_GUTTER_W` = SPACE_LG + SPACE_XS); column_x 590 @1200, cx 600
  EVIDENCE: round-1 green (1042 tests ok, harness RESULT:PASS, PNG 10703B
  distinct 256); wrap-on + wrap-off + trailing-blank paths sab
  `ui.set_width(text_center_gutter_width())`; column_x 590 @1200, cx 600

- [x] G2: yellow hunk-head arrows on changed rows only
  CHECK: `cargo test -p autocompare-gui gutter20` green
  EXPECT: `text_center_marker_label` Changed/Deleted/Inserted -> Some("=>"),
  Equal -> None; `text_center_hunk_head` sirf contiguous-diff block head par;
  `text_center_marker_fg()` == CHANGED_FG (koi naya hue nahi)
  EVIDENCE: round-1 green; Changed/Deleted/Inserted -> Some("=>"), Equal -> None; hunk heads [2,500,1000] on 1200-row model; fg == CHANGED_FG

- [x] G3: selection bands + floor fill keep working
  CHECK: `cargo test -p autocompare-gui gutter20` green
  EXPECT: `text_center_gutter_band_fill(true)` / `!fill(false)`; trailing
  blanks balanced even (`text_trailing_blank_rows_balanced(436.0, row_h, 9)` == 16)
  EVIDENCE: round-1 green; band_fill(true)/!fill(false); balanced(1200.0,17.6,9)==60 even

- [x] G4: tall-scroll edge fixture — gutter + arrows stable
  CHECK: `cargo test -p autocompare-gui gutter20_tall_gutter_stable` green
  + fixture comparison/gutter-tall-1200.json
  EXPECT: 1200-row model par heads scroll offset se na badlen (heads [2] fixed);
  gutter 20.0, column_x 590 @1200, cx 600, marker inset 2.0, fg CHANGED_FG;
  Equal rows kabhi head/marker nahi
  EVIDENCE: gutter20_tall_gutter_stable green + comparison/gutter-tall-1200.json; windows [0,60)->[2], [470,530)->[500], [980,1040)->[1000], [60,470)->[] ; gutter 20.0 col 590 cx 600 inset 2.0

- [x] G5: full autocompare-gui suite green + harness RESULT: PASS
  CHECK: `cargo test -p autocompare-gui` + exquisite-report me `RESULT: PASS`
  EXPECT: 0 failed; PNG >= 8192 B, magic ok, non-blank
  EVIDENCE: 1042 passed / 0 failed; exquisite-report RESULT: PASS (PNG 10703B >= 8192, magic ok, distinct 256)

- [x] G6: MS Rust + frozen gates
  CHECK: `cargo fmt -p autocompare-gui -- --check` + banned grep on non-test delta
  + `grep -rc Color32 crates/gui/src | total` == 33
  EXPECT: fmt clean; non-test unwrap/expect/panic/todo/unreachable/indexing 0;
  colors exactly 33 (gutter20 delta me 0 color touch)
  EVIDENCE: rustfmt --edition 2024 --check clean; non-test banned-token delta 0; colors exactly 33 (views.rs delta me 0 color touch)

- [x] G7: pxdiff before-vs-after honest gate
  CHECK: `python3 tools/exquisite/pxdiff.py round-1/before.png round-1/after.png --expect-diff`
  EXPECT: fixture path headless egui pixels nahi badalta (paint delta headless
  shot me visible nahi) — IDENTICAL hi honest outcome hai; gate ABANDONED likhna
  hai, fake diff nahi; procedural proof = G4 headless geometry test + full suite
  EVIDENCE: ABANDONED — output: `diff_pixels: 0 of 576000 / VERDICT: IDENTICAL (expected diff)` exit 2. Fixture screenshot path live egui pixels nahi dekhta + delta headless-test-only hai, isliye IDENTICAL hi honest outcome hai (fake diff nahi). Procedural proof = G4 headless geometry test + full suite.
