# GATES — exq-tree62 round-1 (leaf)

OWNS: `crates/gui/src/tree.rs` + `checks/gui-exq-tree62/**` only.
GOAL: geometry-first token adoption in tree.rs ONLY — magic-number row
heights/indents/gaps adopt existing SPACE/GEO/TYPO tokens where
value-identical (gutter59 precedent). Zero new tokens/sizes/colors, hit
behavior unchanged.

- [ ] G1: Baseline harness PASS before implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-tree62/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G2: Token delegation present in tree.rs (11 consts ride existing tokens).
  CHECK: `grep -c 'crate::theme::SPACE_LG\|crate::theme::SPACE_SM\|crate::theme::SPACE_XL\|crate::theme::TYPO_BODY_SIZE\|crate::theme::TYPO_MONO_SIZE' crates/gui/src/tree.rs`
  EXPECT: `^([1-9][0-9]+)$`
- [ ] G3: No raw literal const definitions remain for adopted geometry/typo.
  CHECK: `grep -cE 'pub const (FOLDER_EXPANDER_W|FOLDER_ROW_HEIGHT|FOLDER_HEADER_HEIGHT|FOLDER_CELL_PAD_X|FOLDER_COL_GAP|FOLDER_FILTER_BAR_HEIGHT|FOLDER_FILTER_BAR_PAD_X|FOLDER_FILTER_BAR_GAP|FOLDER_BODY_SIZE|FOLDER_SIDE_SIZE|FOLDER_HEADER_SIZE): f32 = [0-9]' crates/gui/src/tree.rs || true`
  EXPECT: `^0$`
- [ ] G4: gui unit tests green (incl. new exq_tree62 value-identical tests).
  CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  EXPECT: `test result: ok`
- [ ] G5: fmt clean.
  CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | tail -1; echo FMT_DONE`
  EXPECT: `FMT_DONE`
- [ ] G6: Banned-call scan 0 in added non-test lines.
  CHECK: `git diff -U0 -- crates/gui/src/tree.rs | grep '^+' | grep -v '^+++' | grep -cE '\.unwrap\(|\.expect\(|panic!|todo!|unreachable!|unsafe' || true`
  EXPECT: `^0$`
- [ ] G7: Harness re-run PASS after implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-tree62/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G8: Live kittest render green (real_egui_shot, App paint proof).
  CHECK: `cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -4`
  EXPECT: `test result: ok`
- [ ] G9: Disjoint OWNS — working tree touches only tree.rs + checks/gui-exq-tree62/.
  CHECK: `git status --short | awk '{print $NF}' | grep -vcE '^(crates/gui/src/tree\.rs|checks/gui-exq-tree62/)' || true`
  EXPECT: `^0$`
- [ ] G10: Render delta documented (after.png size-identical + kittest evidence).
  CHECK: `ls checks/gui-exq-tree62/round-1/RENDER_DELTA.md`
  EXPECT: `checks/gui-exq-tree62/round-1/RENDER_DELTA.md`
