# GATES — exq-gutter59 round-1 (leaf)

OWNS: `crates/gui/src/gutter.rs` + `checks/gui-exq-gutter59/**` only.
GOAL: gutter width/margin/padding alignment adopts SPACE_*/TYPO_* + viewport/canvas
tokens natively; hit behavior byte-identical (painting only); zero new colors/tokens/sizes.

- [ ] G1: Baseline harness PASS before implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-gutter59/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G2: Non-test layout call-sites use canvas tokens, no raw canvas literals.
  CHECK: `python3 checks/gui-exq-gutter59/round-1/raw_canvas_scan.py`
  EXPECT: `RAW_CALLS=0`
- [ ] G3: Token delegation present (canvas + spacing + typo, zero new tokens).
  CHECK: `grep -c 'GUTTER_CANVAS_NORMAL_960\|GUTTER_CANVAS_WIDE_1200\|VIEWPORT_MIN_W' crates/gui/src/gutter.rs`
  EXPECT: `^([4-9]|[1-9][0-9]+)$`
- [ ] G4: gui unit tests green (incl. new exq_gutter59 geometry tests).
  CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  EXPECT: `test result: ok`
- [ ] G5: fmt clean.
  CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | tail -1; echo FMT_DONE`
  EXPECT: `FMT_DONE`
- [ ] G6: Banned-call scan 0 in added non-test lines.
  CHECK: `git diff -U0 -- crates/gui/src/gutter.rs | grep '^+' | grep -v '^+++' | grep -cE '\.unwrap\(|\.expect\(|panic!|todo!|unreachable!|unsafe' || true`
  EXPECT: `^0$`
- [ ] G7: Harness re-run PASS after implementing.
  CHECK: `tools/exquisite/run.sh checks/gui-exq-gutter59/round-1`
  EXPECT: `RESULT: PASS`
- [ ] G8: Live kittest render green (real_egui_shot, App paint proof).
  CHECK: `cargo test -p autocompare-gui --test real_egui_shot 2>&1 | tail -4`
  EXPECT: `test result: ok`
- [ ] G9: Disjoint OWNS — working tree touches only gutter.rs + checks/gui-exq-gutter59/.
  CHECK: `git status --short | awk '{print $NF}' | grep -vcE '^(crates/gui/src/gutter\.rs|checks/gui-exq-gutter59/)' || true`
  EXPECT: `^0$`
- [ ] G10: Render delta documented (after.png vs real_egui_shot).
  CHECK: `ls checks/gui-exq-gutter59/round-1/RENDER_DELTA.md`
  EXPECT: `checks/gui-exq-gutter59/round-1/RENDER_DELTA.md`
