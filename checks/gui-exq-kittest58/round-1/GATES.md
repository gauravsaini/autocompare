# GATES — exq-kittest58 round-1: dual-theme real-egui shots (test-only)

OWNS: `crates/gui/tests/real_egui_shot.rs`, `checks/gui-exq-kittest58/**`

Scope: harden screenshot pipeline with a light-theme second shot; zero app change.

RULES: Hinglish-Roman chat, English-only commits. Never push. Touch NO other
crates/gui files (theme.rs typo54, views.rs views57) and NO other checks dirs.
Test-only diff. forbid unsafe. Prefer Result<String> returns (existing style).

- [x] G1 Baseline exquisite harness PASS on fresh round dir (pre-change)
  CHECK: tools/exquisite/run.sh checks/gui-exq-kittest58/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: 2026-11-21 exit 0, report RESULT: PASS (PNG 10946B magic ok distinct 256)

- [x] G2 Dual-theme tests green (dark + light, same fixtures/size)
  CHECK: cargo test -p autocompare-gui --test real_egui_shot
  EXPECT: test result: ok
  EVIDENCE: 2026-11-21 2 passed 0 failed (real_egui_text_compare_shot, ..._light)

- [x] G3 Both PNGs meet floors in target tmp (bytes/dims/distinct-RGB/AA ratio)
  CHECK: cargo test -p autocompare-gui --test real_egui_shot -- --nocapture | grep -E 'real-egui\(|test result'
  EXPECT: real-egui(real-egui-light.png)
  EVIDENCE: dark bytes=51926 colors=503 base=10 ratio=50; light bytes=52669 colors=549 base=10 ratio=54; both 960x600 magic ok (>=8192B, >=500 colors, ratio>=10)

- [x] G4 fmt clean on OWNS file, banned tokens 0, disjoint OWNS
  CHECK: rustfmt --check crates/gui/tests/real_egui_shot.rs && git status --short
  EXPECT: OWNS-FMT-OK
  EVIDENCE: 2026-11-21 rustfmt clean; grep unwrap|expect(|panic!|todo!|unsafe in test file hits only forbid(unsafe_code) attr; git status shows only OWNS paths (test file + checks/gui-exq-kittest58/); strays in checks/gui-sessionpaint-by reverted

- [x] G5 Harness re-run PASS post-change, report documents dark+light stats
  CHECK: tools/exquisite/run.sh checks/gui-exq-kittest58/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: 2026-11-21 exit 0, report RESULT: PASS; dual-shot section appended to exquisite-report.md

## EVIDENCE (measured 2026-11-21, single test run + tmp PNGs)
- dark (real-egui.png): bytes 51926, dims 960x600, distinct-RGB 503, base 10, ratio 50
- light (real-egui-light.png): bytes 52669, dims 960x600, distinct-RGB 549, base 10, ratio 54
- floors: MIN_PNG_BYTES 8192 ok, MIN_DISTINCT_RGB 500 ok, AA_RATIO 10 ok
- note: tmp PNG bytes are nondeterministic across runs (live clock repaint);
  floors hold every run; tracked checks/ copies refresh only on EXQ_KEEP_PNG=1
