# GATES: BN-GEO folder+text geometry tight (BC parity)

OWNS: crates/gui/src/views.rs, crates/gui/src/views_table.rs, checks/gui-geo-bn/
NO-TOUCH: app.rs, theme.rs, gutter.rs, home.rs, tree.rs, screenshot.rs, viewmodel.rs, checks/opencode-waves/STATUS.md, checks/gui-typo-bn/, checks/bin-bo/, checks/localrun-bo/

Scope: Folder + Text views ki geometry BC parity tak tight karo. Colors FROZEN (koi Color32/Visuals edit nahi). Typography sirf layout-ke-liye zaroori minimum.

## Observable gates (sab CHECK/EXPECT/EVIDENCE ke saath)

- [x] G1: header single compact row (2-line waste khatam)
  CHECK: `grep -n "paint_header" crates/gui/src/views.rs` + headless render `target/debug/autocompare-gui --screenshot checks/gui-geo-bn/round-1/after.png` me header 1 row dikhe
  EXPECT: title + small one-line hint same row me, neeche koi second note line nahi, gap TIGHT_GAP
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G2: tab strip tight single flow
  CHECK: `grep -n "paint_tabs" -A 12 crates/gui/src/views.rs | grep -c "horizontal_wrapped"` == 0 aur `add_space(TIGHT_GAP)` present
  EXPECT: tabs ek row me, wrap nahi, bottom gap 6 se 4 par
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G3: path bar single row fixed height
  CHECK: `grep -n "paint_paths" -A 40 crates/gui/src/views.rs` me Compare button same horizontal me, `set_min_size(.*HEADER_H)` present
  EXPECT: left field + Browse + handle + right field + Browse + Compare ek row me, button height 28 reuse
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G4: toolbar single flow consistent padding
  CHECK: `grep -n "horizontal_wrapped" crates/gui/src/views.rs crates/gui/src/views_table.rs` == 0 (toolbar/tag sites par)
  EXPECT: text toolbar + key/find bars single horizontal, item_spacing TIGHT_GAP scale
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G5: folder results Grid Name/Size/Modified + fixed row height + text_style_height
  CHECK: `grep -n "Grid::new" crates/gui/src/views.rs` >= 4 aur `grep -n "text_style_height" crates/gui/src/views.rs` >= 3 aur folder grid loop me `set_row_height(row_height)` present
  EXPECT: folder/sync/dual/table/hex sab Grid me, har row fixed `row_height_for_line(text_row_height(ui))` par
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G6: ScrollArea ID stable, debug red overlap khatam
  CHECK: `grep -n "ScrollArea" crates/gui/src/views.rs | grep -c "id_salt"` == ScrollArea count, screenshot me koi red debug text overlap nahi
  EXPECT: har ScrollArea par unique id_salt, ek hi Compose me 3 ScrollArea hone par bhi koi id clash nahi
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G7: row text overlap fix (Grid/columns, no manual x)
  CHECK: folder path cell me Grid columns + `set_row_height`, `tree_indent` token ke bahar koi raw `add_space(<n>)` offset nahi
  EXPECT: lambi path par bhi size/time columns align, glyph/path overlap nahi
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G8: harness RESULT:PASS (max 6 rounds fix-and-repeat)
  CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-geo-bn/round-1/`
  EXPECT: `RESULT: PASS` + PNG magic ok + size >= 8192 + distinct > 64
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G9: cargo test green
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: test result ok, 0 failed
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G10: clippy zero warnings
  CHECK: `cargo clippy --all-targets -- -D warnings`
  EXPECT: exit 0, koi warning nahi
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G11: rustfmt clean (sirf OWNS rs files)
  CHECK: `cargo fmt --version && rustfmt --edition 2024 --check crates/gui/src/views.rs crates/gui/src/views_table.rs`
  EXPECT: exit 0, koi diff nahi
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G12: banned + colors grep zero new
  CHECK: `grep -rn "unwrap()\|expect(\|panic!(\|todo!(\|todo_macro" crates/gui/src/views.rs crates/gui/src/views_table.rs | grep -v "^\s*//" | grep -v test ; echo BANNED:$?` aur baseline `git diff` me koi `Color32::from_rgb` / `Visuals::` add nahi
  EXPECT: banned zero, colors diff zero
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)

- [x] G13: STATUS untouched
  CHECK: `git status --short checks/opencode-waves/STATUS.md` empty
  EXPECT: koi modify nahi
  EVIDENCE: round-1 green (607 tests ok, clippy 0, fmt clean, harness RESULT:PASS, PNG 9695B distinct 255)
