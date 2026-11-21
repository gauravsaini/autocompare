# GATES — exq-tree82 round-1 (leaf)

OWNS: `crates/gui/src/tree.rs` + `checks/gui-exq-tree82/**` only.
GOAL: tree geometry token adoption in tree.rs ONLY — FOLDER_FILTER_COMBO_PAD
rides existing SPACE_XL (value-identical 24.0, home CTA frame parity).
Zero new tokens/sizes/colors, hit behavior unchanged, typography frozen.

- [ ] G1: GATES written before baseline.
  CHECK: `ls checks/gui-exq-tree82/round-1/GATES.md`
  EXPECT: `checks/gui-exq-tree82/round-1/GATES.md`
- [ ] G2: Baseline gui unit tests green before implementing.
  CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  EXPECT: `test result: ok`
- [ ] G3: COMBO_PAD rides SPACE_XL token (no raw 24.0 literal).
  CHECK: `grep -c 'pub const FOLDER_FILTER_COMBO_PAD: f32 = crate::theme::SPACE_XL' crates/gui/src/tree.rs`
  EXPECT: `^1$`
- [ ] G4: gui unit tests green after implementing (incl. combo-pad parity test).
  CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  EXPECT: `test result: ok`
- [ ] G5: fmt clean.
  CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | tail -1; echo FMT_DONE`
  EXPECT: `FMT_DONE`
- [ ] G6: Banned-call scan 0 in added non-test lines.
  CHECK: `git diff -U0 -- crates/gui/src/tree.rs | grep '^+' | grep -v '^+++' | grep -cE '\.unwrap\(|\.expect\(|panic!|todo!|unreachable!|unsafe' || true`
  EXPECT: `^0$`
- [ ] G7: Harness PASS after implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-tree82/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G8: Disjoint OWNS — working tree touches only tree.rs + checks/gui-exq-tree82/.
  CHECK: `git status --short | awk '{print $NF}' | grep -vcE '^(crates/gui/src/tree\.rs|checks/gui-exq-tree82/)' || true`
  EXPECT: `^0$`
- [ ] G9: No strays in frozen dirs (sessionpaint-by, sessions untouched).
  CHECK: `git status --short | grep -cE 'gui-sessionpaint-by|crates/sessions' || true`
  EXPECT: `^0$`
