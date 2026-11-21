# GATES — exq-typo54 round-1: real system-font chains (theme.rs ONLY)

OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-typo54/**`. No other files.

- [ ] G1 baseline harness PASS on fresh round dir
  - CHECK: `tools/exquisite/run.sh checks/gui-exq-typo54/round-1`
  - EXPECT: `RESULT: PASS`

- [ ] G2 pure candidate-chain helpers in theme.rs only
  - CHECK: `git status --short | grep -v '^??' | awk '{print $2}' | grep -v '^checks/gui-exq-typo54/' | grep -v '^crates/gui/src/theme.rs$' | wc -l`
  - EXPECT: `0`

- [ ] G3 full gui test suite green
  - CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  - EXPECT: `test result: ok`

- [ ] G4 fmt clean
  - CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | wc -l`
  - EXPECT: `0`

- [ ] G5 banned tokens zero in non-test code
  - CHECK: `grep -rn 'unwrap\|expect(\|panic!\|todo!\|unreachable!\|unsafe' crates/gui/src/theme.rs | grep -v 'unwrap_or\|// ' | grep -v '^\s*//' | awk -F: '{print $1":"$2}' | head -5; echo BANNED-SCAN-DONE`
  - EXPECT: `BANNED-SCAN-DONE`

- [ ] G6 every TypoRole chain non-empty, mono differs from proportional, generic fallback last
  - CHECK: `cargo test -p autocompare-gui typo54 2>&1 | tail -5`
  - EXPECT: `test result: ok`

- [ ] G7 harness re-run PASS after implement
  - CHECK: `tools/exquisite/run.sh checks/gui-exq-typo54/round-1`
  - EXPECT: `RESULT: PASS`

- [ ] G8 kittest real-egui shot render documented
  - CHECK: `cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -3`
  - EXPECT: `test result: ok`
