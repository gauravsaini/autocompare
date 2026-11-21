# GATES — exq-fontbundle67 round-1: typo consolidation in theme.rs ONLY

OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-fontbundle67/**`. No other files.

Scope: replace magic font-size/weight/line-height numbers with existing TYPO_*
reuse where value-identical (letter_spacing 0.0 sites), tighten TYPO54 chain
docs, no new sizes/weights/colors, behavior unchanged.

- [x] G1 baseline harness PASS on fresh round dir (pre-change)
  CHECK: tools/exquisite/run.sh checks/gui-exq-fontbundle67/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: 2026-11-21 exit 0, report RESULT: PASS (PNG 10946B magic ok distinct 256)

- [x] G2 disjoint OWNS — only theme.rs + checks/gui-exq-fontbundle67 change
  CHECK: git status --short | grep -v '^??' | awk '{print $2}' | grep -v '^checks/gui-exq-fontbundle67/' | grep -v '^crates/gui/src/theme.rs$' | wc -l
  EXPECT: 0
  EVIDENCE: 2026-11-21 count 0 after reverting harness-regenerated sessionpaint-by PNG strays

- [x] G3 full gui test suite green
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 lib 1130 passed 0 failed; integration targets ok

- [x] G4 fmt clean
  CHECK: cargo fmt -p autocompare-gui -- --check 2>&1 | wc -l
  EXPECT: 0
  EVIDENCE: 2026-11-21 output 0 lines

- [x] G5 banned tokens zero in non-test code
  CHECK: sed '/^#\[cfg(test)\]/,$d' crates/gui/src/theme.rs | grep -n 'unwrap\|expect(\|panic!\|todo!\|unreachable!\|unsafe' | grep -v 'unwrap_or' | grep -v 'forbid(unsafe_code)' | wc -l
  EXPECT: 0
  EVIDENCE: 2026-11-21 count 0 (sole raw hit is the `forbid(unsafe_code)` attr)

- [x] G6 typo-reuse unit tests green (value-identical locks)
  CHECK: cargo test -p autocompare-gui fontbundle67 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 theme::tests::fontbundle67_typo_reuse_value_identical ok (1 passed)

- [x] G7 harness re-run PASS after implement
  CHECK: tools/exquisite/run.sh checks/gui-exq-fontbundle67/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: 2026-11-21 exit 0, RESULT: PASS (PNG 10946B, colors touches 35 unchanged, TypoRole x1182 = +8 from new test)

- [x] G8 kittest real-egui shot render documented
  CHECK: cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 2 passed 0 failed; dark real-egui.png 53663B 503 colors, light 54289B 549 colors, 960x600, AA ratio 50/54 (>=10)
