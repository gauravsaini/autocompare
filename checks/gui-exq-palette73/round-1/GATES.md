# GATES: exq-palette73 round-1 residual palette wiring

OWNS: crates/gui/src/views.rs, checks/gui-exq-palette73/**

Scope: views.rs ONLY — 4 hardcoded dark washes (lines ~2407,2461,3002,6270)
route via existing theme *_for(dark). Zero new Color32, geometry FROZEN.

- [x] G1: Baseline exquisite harness PASS before edit
  CHECK: tools/exquisite/run.sh checks/gui-exq-palette73/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: RESULT: PASS (PNG 10946B, magic ok, distinct 256) pre-edit run EXIT 0.

- [x] G2: 4 residual sites route via *_for(dark), zero hardcoded washes in paint paths
  CHECK: python3 -c "import subprocess; out=subprocess.run(['grep','-n','CHANGED_BG\|INSERTED_BG\|DELETED_BG','crates/gui/src/views.rs'],capture_output=True,text=True); paint=[l for l in out.stdout.splitlines() if 'rect_filled' in l or '=> CHANGED_BG' in l or '=> INSERTED_BG' in l or '=> DELETED_BG' in l]; print(len(paint)); raise SystemExit(0 if len(paint)==0 else 1)"
  EXPECT: 0
  EVIDENCE: 0 — grep shows BG consts only in imports-for-tests, doc links (crate::theme-qualified), and test asserts; no rect_filled/match-arm use.

- [x] G3: Unit tests prove dark byte-identical + light wash AA
  CHECK: cargo test -p autocompare-gui exq_palette73_residual -- --nocapture
  EXPECT: test result: ok
  EVIDENCE: views::tests::exq_palette73_residual ok (1 passed); dark arms == CHANGED_BG/INSERTED_BG/DELETED_BG, light arms == LIGHT_*_BG + AA.

- [x] G4: Full gui suite green + fmt clean + banned scan 0 in non-test diff
  CHECK: cargo test -p autocompare-gui
  EXPECT: test result: ok
  EVIDENCE: 1139 passed 0 failed (lib) + 2 passed real_egui_shot; banned scan BANNED_0 on added diff lines.

- [x] G5: fmt + banned tokens clean
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: clean
  EVIDENCE: FMT_CLEAN after cargo fmt -p autocompare-gui.

- [x] G6: Harness re-run PASS + kittest documented (no live-paint claim from after.png)
  CHECK: tools/exquisite/run.sh checks/gui-exq-palette73/round-1 && cargo test -p autocompare-gui --test real_egui_shot
  EXPECT: RESULT: PASS
  EVIDENCE: RESULT: PASS post-edit (PNG 10946B identical size); real_egui_shot 2 passed (text_compare_shot + light). after.png is headless --screenshot artifact only — no live-paint claim.
