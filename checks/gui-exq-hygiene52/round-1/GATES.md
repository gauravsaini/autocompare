# GATES — exq-hygiene52 round-1 (test-output hygiene)

OWNS: `crates/gui/tests/real_egui_shot.rs`, `crates/diff/src/image.rs`
Context: (1) `real_egui_shot` test rewrites tracked
`checks/gui-exq-kittest49/round-1/real-egui.png` on EVERY `cargo test`
run (non-deterministic bytes, dirties tree). (2) wgpu readback frame
carries alpha 0 on a fully-covered opaque window; `encode_png` is a
pass-through so the alpha-0 lands in the PNG (viewers tolerate, not honest).
Fix: shot goes to target-tmp by default, `checks/` copy only on
`EXQ_KEEP_PNG=1`; capture site forces opaque alpha 255 via a new
`RgbaImage::opaque()` helper (RGB untouched, colors frozen). Assertions
(magic/size/AA) unchanged.

- [x] G1 baseline harness PASS before change
  CHECK: tools/exquisite/run.sh checks/gui-exq-hygiene52/round-1 | tail -5
  EXPECT: RESULT: PASS
- [x] G2 default test run leaves owned checks/ file untouched
  (sessionpaint-by strays are out-of-scope, owned by views.rs tests)
  CHECK: cargo test -p autocompare-gui --quiet; git status --porcelain -- checks/gui-exq-kittest49/round-1/real-egui.png | wc -l
  EXPECT: ^0$
- [x] G3 EXQ_KEEP_PNG=1 restores the checks/ copy path
  CHECK: EXQ_KEEP_PNG=1 cargo test -p autocompare-gui --quiet real_egui && ls -la checks/gui-exq-kittest49/round-1/real-egui.png
  EXPECT: real-egui.png
- [x] G4 test assertions unchanged (magic/size/AA still enforced)
  CHECK: grep -c MIN_PNG_BYTES\|MIN_DISTINCT_RGB\|AA_RATIO crates/gui/tests/real_egui_shot.rs
  EXPECT: ^[4-9]$
- [x] G5 opaque alpha unit test green (RGB untouched, alpha 255, roundtrip honest)
  CHECK: cargo test -p autocompare-diff --quiet opaque
  EXPECT: test result: ok
- [x] G6 full gui suite green twice, owned paths clean (strays reverted per rule)
  CHECK: cargo test -p autocompare-gui --quiet && cargo test -p autocompare-gui --quiet; git checkout -- checks/gui-sessionpaint-by/; git status --porcelain -- ':!checks/gui-exq-hygiene52' | wc -l
  EXPECT: ^0$
- [x] G7a banned scan zero in OWNS (no unsafe/unwrap/expect/panic/todo;
  the single `unsafe` hit is the `#![forbid(unsafe_code)]` header)
  CHECK: grep -rnE 'unsafe|unwrap\(|expect\(|panic!|todo!' crates/diff/src/image.rs crates/gui/tests/real_egui_shot.rs | grep -v 'forbid(unsafe_code)' | wc -l
  EXPECT: ^0$
- [x] G7b fmt: OWNS files clean; repo-wide drift confined to sessions/*
  (manual; pre-existing at base, untouched per `git diff --stat`)
  CHECK: cargo fmt --check 2>&1 | grep '^Diff'
  EXPECT: sessions only, zero hits for diff/src/image.rs or real_egui_shot.rs
- [x] G8 harness PASS after change, byte behavior documented
  CHECK: tools/exquisite/run.sh checks/gui-exq-hygiene52/round-1 | tail -5
  EXPECT: RESULT: PASS
