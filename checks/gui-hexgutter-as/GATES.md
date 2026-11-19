# Wave AS-HEXGUTTER — GATES (hex gutter parity: offsets/row markers + click-to-jump)

OWNS: `crates/gui/src/views_hex.rs` + `crates/gui/src/gutter.rs` + `checks/gui-hexgutter-as/**` ONLY.
KEVL `views.rs`/`app.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`home.rs`/`theme.rs`/`screenshot.rs` mat chhoona (read bhi avoid — sirf `theme.rs` token grep, `viewmodel.rs` kind grep).
Colors FROZEN (koi hue change nahi — koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi, koi `.size(` add nahi).
Geometry first, typography second. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.
Clean-room: Beyond Compare bahar se observe only, apne shabdon me.

Scope: hex view me text gutter precedent par gutter parity — offsets/row markers + click-to-jump.
`gutter.rs` helpers reuse (`gutter_bar_width`, `gutter_bar_width_for_font`, `gutter_row_at_y`,
`gutter_segment_at_y`, `gutter_y_for_row`, `GutterGeometry`/`GutterSegment`, typo role/size/source).
`views_hex.rs` me nayi pure hex-gutter layer: `hex_gutter_total_rows`, `hex_gutter_geometry`
(same min-height/merge-gap tokens, kind `Changed`), thin delegates
(`hex_gutter_bar_width*`, `hex_gutter_text_*`, `hex_gutter_row_at_y`/`segment_at_y`/`y_for_row`),
`hex_gutter_jump_target` (y -> row -> covering hunk, forward phir backward snap),
`hex_gutter_row_label` (row marker = `format_hex_offset(hex_row_start(row))`).
Koi egui/I/O nahi — paint wiring orchestrator ke paas, ye sirf headless math hai.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `473 passed; 0 failed` (recorded 2026-11-20, `main@c9a5c18`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views_hex|gutter).rs|checks/gui-hexgutter-as/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|app.rs|viewmodel.rs|tree.rs|views_table.rs|home.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_hex.rs crates/gui/src/gutter.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_hex.rs` (+`gutter.rs` agar zaroorat pade, warna untouched) + `checks/gui-hexgutter-as/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Hex gutter parity, pure helpers + reuse, unit-tested

- CHECK: `cargo test -p autocompare-gui hex_gutter`
- EXPECT: green, kam se kam ye tests:
  + `hex_gutter_geometry_marks_diff_rows_and_merges` (2 diff rows merge, equal gap khali, `Changed` kind, min-height/merge-gap same tokens)
  + `hex_gutter_width_and_typo_follow_text_precedent` (bar widths + typo role/size/source `gutter.rs` ke barabar)
  + `hex_gutter_row_at_y_delegates_to_text_mapping` (band mapping + reject cases same)
  + `hex_gutter_jump_target_snaps_to_covering_hunk` (covering hunk, forward snap, backward snap, khali/oor None)
  + `hex_gutter_row_labels_use_offset_ruler` (`hex_gutter_row_label(r) == format_hex_offset(hex_row_start(r))`)
- WHAT: `views_hex.rs` me naya `Hex gutter parity` section (sab `#[allow(dead_code)]`, test-only until paint wiring lands):
  `hex_gutter_total_rows` (`hex_row_count` reuse), `hex_gutter_geometry` (`hex_row_has_diff` + same
  `GUTTER_MIN_SEGMENT_H`/`GUTTER_MERGE_GAP`, `GutterGeometry` reuse), delegates
  (`hex_gutter_bar_width`, `hex_gutter_bar_width_for_font`, `hex_gutter_text_role/size/line_height_source`,
  `hex_gutter_row_at_y`, `hex_gutter_segment_at_y`, `hex_gutter_y_for_row`), `hex_gutter_jump_target`
  (`gutter_row_at_y` + `hex_row_to_hunk`, forward phir backward row scan), `hex_gutter_row_label`
  (`format_hex_offset` + `hex_row_start` reuse). Koi naya token/color nahi.

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views_hex.rs crates/gui/src/gutter.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_hex.rs crates/gui/src/gutter.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views_hex.rs crates/gui/src/gutter.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hexgutter-as/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-hexgutter-as/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-hexgutter-as/REPORT.md checks/gui-hexgutter-as/round-1/after.png checks/gui-hexgutter-as/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_hex.rs crates/gui/src/gutter.rs checks/gui-hexgutter-as/GATES.md checks/gui-hexgutter-as/REPORT.md checks/gui-hexgutter-as/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
  (agar `gutter.rs` untouched raha to `git add` me uska naam hone par bhi staged nahi hoga — sirf OWNS paths)
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
