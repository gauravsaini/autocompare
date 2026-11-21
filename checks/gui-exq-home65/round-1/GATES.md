# GATES exq-home65 round-1 — geometry-first token adoption in home.rs

OWNS: crates/gui/src/home.rs + checks/gui-exq-home65/** ONLY.
NO-TOUCH: views_hex.rs (hex64 live), all other crates/gui files, all other checks dirs.
GOAL: replace magic-number card paddings/row heights/gaps with existing
SPACE/GEO tokens where value-identical (gutter59/table63 precedent).
Zero new tokens/sizes/colors, hit behavior unchanged.
Colors FROZEN: zero new Color32/Visuals.

## Gates

- [ ] G1 baseline harness PASS (pre-implement, fresh round dir)
  CHECK: tools/exquisite/run.sh checks/gui-exq-home65/round-1 2>&1 | tail -5
  EXPECT: RESULT: PASS

- [ ] G2 token adoption at 7 value-identical call sites (literals -> tokens)
  CHECK: grep -n "HOME_CARD_MIN_W: f32 = \|HOME_TOOLBAR_BUTTON_H: f32 = \|HOME_CARD_CTA_FRAME_PAD: f32 = \|RECENT_COL_KIND_W: f32 = \|RECENT_COL_AGE_W: f32 = \|SESSION_TREE_EXPANDER_W: f32 = \|SESSION_TREE_ICON_W: f32 = " crates/gui/src/home.rs
  EXPECT: SPACE_XL

- [ ] G3 full gui test suite green (incl. new exq_home65 value-identical test)
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: ok

- [ ] G4 fmt clean + banned scan zero in non-test code
  CHECK: cargo fmt -p autocompare-gui -- --check 2>&1 | tail -2; echo FMT_DONE; git diff -U0 -- crates/gui/src/home.rs | grep '^+' | grep -v '^+++' | grep -cE '\.unwrap\(|\.expect\(|panic!|todo!|unreachable!|unsafe' || true
  EXPECT: FMT_DONE

- [ ] G5 harness re-run PASS after implement (after.png size-identical expected)
  CHECK: tools/exquisite/run.sh checks/gui-exq-home65/round-1 2>&1 | tail -5
  EXPECT: RESULT: PASS

- [ ] G6 kittest render green (real_egui_shot proof, DOCUMENTED delta note)
  CHECK: cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -4
  EXPECT: test result: ok

- [ ] G7 commit OWNS only, English conventional, no push
  CHECK: git status --short | head -10
  EXPECT: home.rs
