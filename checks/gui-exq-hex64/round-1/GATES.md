# GATES — exq-hex64 round-1 (leaf)

OWNS: `crates/gui/src/views_hex.rs` + `checks/gui-exq-hex64/**` only.
GOAL: geometry-first token adoption in views_hex.rs ONLY — magic-number
byte-cell sizes/row heights/gaps adopt existing SPACE/GEO/GUTTER/TYPO
(+ canonical views::FIND_NEEDLE_W / views::HEX_MINIMAP_H) tokens where
value-identical (repo-wide grep per call-site, gutter59/tree62 precedent).
Zero new tokens/sizes/colors, hit behavior unchanged.

- [ ] G1: Baseline harness PASS before implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-hex64/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G2: Token delegation present in views_hex.rs (14 value-identical adoptions).
  CHECK: `grep -c 'crate::theme::CHROME_HEADER_BAR_H\|crate::views::FIND_NEEDLE_W\|crate::theme::GEO_TIGHT_GAP\|crate::views::HEX_MINIMAP_H\|crate::theme::VIEWPORT_W as f32\|crate::theme::VIEWPORT_H as f32\|crate::theme::VIEWPORT_MIN_W as f32\|crate::gutter::GUTTER_CANVAS_WIDE_1200\|crate::gutter::GUTTER_CANVAS_NARROW_700' crates/gui/src/views_hex.rs`
  EXPECT: `^([1-9][0-9]+)$`
- [ ] G3: No raw literal const definitions remain for adopted geometry.
  CHECK: `grep -cE 'pub const (HEX_FIND_BAR_H|HEX_FIND_NEEDLE_W|HEX_FIND_BADGE_PAD|HEX_CANVAS_W|HEX_CANVAS_H|HEX_STACK_BREAKPOINT_W|HEX_TALL_CANVAS_H|HEX_NARROW_CANVAS_W|HEX_NARROW_CANVAS_H|HEX_WIDE1200_CANVAS_W|HEX_WIDE1200_CANVAS_H|HEX_NARROW700_CANVAS_W|HEX_NARROW700_CANVAS_H): f32 = [0-9]' crates/gui/src/views_hex.rs || true`
  EXPECT: `^0$`
- [ ] G4: gui unit tests green (incl. new exq_hex64 value-identical tests).
  CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  EXPECT: `test result: ok`
- [ ] G5: fmt clean.
  CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | tail -1; echo FMT_DONE`
  EXPECT: `FMT_DONE`
- [ ] G6: Banned-call scan 0 in added non-test lines.
  CHECK: `git diff -U0 -- crates/gui/src/views_hex.rs | grep '^+' | grep -v '^+++' | grep -cE '\.unwrap\(|\.expect\(|panic!|todo!|unreachable!|unsafe' || true`
  EXPECT: `^0$`
- [ ] G7: Harness re-run PASS after implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-hex64/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G8: Live kittest render green (real_egui_shot, App paint proof).
  CHECK: `cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -4`
  EXPECT: `test result: ok`
- [ ] G9: Disjoint OWNS — working tree touches only views_hex.rs + checks/gui-exq-hex64/.
  CHECK: `git status --short | awk '{print $NF}' | grep -vcE '^(crates/gui/src/views_hex\.rs|checks/gui-exq-hex64/)' || true`
  EXPECT: `^0$`
- [ ] G10: Render delta documented (after.png size-identical + kittest evidence).
  CHECK: `ls checks/gui-exq-hex64/round-1/RENDER_DELTA.md`
  EXPECT: `checks/gui-exq-hex64/round-1/RENDER_DELTA.md`
