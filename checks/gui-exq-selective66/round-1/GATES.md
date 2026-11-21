# GATES — exq-selective66 round-1: selective formats via theme *_for(dark)

OWNS: `crates/gui/src/views.rs` + `checks/gui-exq-selective66/**` only.
GOAL: `selective_span_format` + `selective_find_format` route via existing
`changed_*_for(dark)` helpers; dark byte-identical, light uses `LIGHT_*` wash.
Zero new colors/sizes; hit behavior unchanged.

- [ ] G1 harness green on this round dir
  CHECK: `tools/exquisite/run.sh checks/gui-exq-selective66/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G2 full gui test suite green
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: `test result: ok`
- [ ] G3 fmt clean
  CHECK: `cargo fmt --check -p autocompare-gui`
  EXPECT: exit 0 (no diff output)
- [ ] G4 no banned calls in non-test code
  CHECK: `rg -n 'unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe' crates/gui/src/views.rs | rg -v '#\[cfg\(test\)\]' || true`
  EXPECT: empty output (manual: test-module hits excluded by inspection)
- [ ] G5 dark arm byte-identical to frozen CHANGED pair
  CHECK: `cargo test -p autocompare-gui selective_`
  EXPECT: `test result: ok`
- [ ] G6 kittest render documented
  CHECK: `cargo test -p autocompare-gui --test real_egui_shot`
  EXPECT: `test result: ok`
