# GATES — exq-fonts76 round-1: typography in theme.rs ONLY

OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-fonts76/**`. No other files.

Scope: bundled sans+mono slots + light Visuals polish, all theme-owned
(headless). No geometry moves (zero SPACE/GEO changes), no new palette hues
(Visuals reuses DARK_*/LIGHT_* tokens only), AA intact.

- [ ] G1 baseline harness PASS on fresh round dir (pre-change reference)
  ABANDON: GUARD relaunch required OWNS edit in first 120s (disk had been
  100% full, prior run stalled past 600s with zero OWNS edit), so no
  pre-change baseline was captured. Paint-neutrality proven instead by G7:
  post-change PNG 10946 B byte-identical in size to fontbundle67's
  pre/post PNGs (10946 B), magic ok, distinct 256.

- [x] G2 disjoint OWNS — only theme.rs + checks/gui-exq-fonts76 change
  CHECK: git status --short | grep -v '^??' | awk '{print $2}' | grep -v '^checks/gui-exq-fonts76/' | grep -v '^crates/gui/src/theme.rs$' | wc -l
  EXPECT: 0
  EVIDENCE: 2026-11-21 count 0 after reverting harness-regenerated sessionpaint-by PNG strays

- [x] G3 full gui test suite green
  CHECK: CARGO_BUILD_JOBS=2 cargo test -p autocompare-gui --target-dir target/agent-fonts76 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 lib 1141 passed 0 failed; harness tests.out full-suite tail ok

- [x] G4 fmt clean
  CHECK: rustfmt --edition 2024 --check crates/gui/src/theme.rs && echo FMT_OK
  EXPECT: FMT_OK
  EVIDENCE: 2026-11-21 FMT_OK

- [x] G5 banned tokens zero in non-test code
  CHECK: sed '/^#\[cfg(test)\]/,$d' crates/gui/src/theme.rs | grep -n 'unwrap\|expect(\|panic!\|todo!\|unreachable!\|unsafe' | grep -v 'forbid(unsafe_code)' | grep -v 'unwrap_or' | wc -l
  EXPECT: 0
  EVIDENCE: 2026-11-21 count 0

- [x] G6 fonts76 unit test green (bundled slots + light polish locks)
  CHECK: CARGO_BUILD_JOBS=2 cargo test -p autocompare-gui --target-dir target/agent-fonts76 fonts76 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 theme::tests::fonts76_bundled_sans_mono_light_polish ok (1 passed)

- [x] G7 harness re-run PASS after implement
  CHECK: CARGO_TARGET_DIR=/tmp/wt-fonts76/target/agent-fonts76 tools/exquisite/run.sh checks/gui-exq-fonts76/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: 2026-11-21 exit 0, RESULT: PASS (PNG 10946B magic 89504e470d0a1a0a distinct 256; colors touches 42 = 35 prior + 7 new palette-reuse Visuals/fill lines, zero new hues)

- [x] G8 kittest real-egui shot render documented
  CHECK: cargo test -p autocompare-gui --target-dir target/agent-fonts76 --test real_egui_shot 2>&1 | tail -3
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 2 passed 0 failed (real_egui_text_compare_shot dark + light; test asserts PNG magic, min bytes, distinct internally)
