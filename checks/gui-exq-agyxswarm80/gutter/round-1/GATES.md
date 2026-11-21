# GATES — exq-agyxswarm80 gutter round-1 (leaf)

OWNS: `crates/gui/src/gutter.rs` + `checks/gui-exq-agyxswarm80/gutter/**` only.
GOAL: Gutter geometry reuse SPACE_* tokens natively in `crates/gui/src/gutter.rs`, colors FROZEN (no Color32 touches, no palette changes), 0 banned constructs in non-test code.

- [x] G1: Baseline test PASS before implementing.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: `test result: ok`
- [x] G2: Gutter geometry reuses `SPACE_*` tokens (`SPACE_XS`, `SPACE_SM`, `SPACE_MD`, `SPACE_LG`, `SPACE_XL`) in `crates/gui/src/gutter.rs`.
  CHECK: `cargo test -p autocompare-gui -- exq_agyxswarm80_gutter_tests`
  EXPECT: `test result: ok`
- [x] G3: Colors FROZEN (no Color32 touches, no palette changes).
  CHECK: `git diff crates/gui/src/gutter.rs | grep -i Color32 || true`
  EXPECT: `^$`
- [x] G4: Zero banned constructs in non-test code (pure safe Rust).
  CHECK: `git diff -U0 crates/gui/src/gutter.rs | grep '^+' | grep -v '^+++' | grep -cE 'unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe ' || true`
  EXPECT: `^0$`
- [x] G5: All GUI tests green.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: `test result: ok`
- [x] G6: `rustfmt --check crates/gui/src/gutter.rs` clean.
  CHECK: `rustfmt --check crates/gui/src/gutter.rs`
  EXPECT: `0`
- [x] G7: Disjoint OWNS — working tree touches only owned files.
  CHECK: `git status --short | awk '{print $NF}' | grep -vcE '^(crates/gui/src/gutter\.rs|checks/gui-exq-agyxswarm80/gutter/)' || true`
  EXPECT: `^0$`
