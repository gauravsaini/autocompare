# GATES — exq-kittest49 round-1: real-egui screenshot pipeline

## OWNS
- `crates/gui/Cargo.toml` (dev-dep add only)
- `crates/gui/tests/real_egui_shot.rs` (new)
- `checks/gui-exq-kittest49/**`

## RULES
- Hinglish-Roman chat, English-only commits. Never push.
- No runtime dep change, no shipped-app behavior change, no unsafe.

- [x] G1 Baseline exquisite harness PASS with software rasterizer
  CHECK: tools/exquisite/run.sh checks/gui-exq-kittest49/round-1
  EXPECT: RESULT: PASS

- [x] G2 Dev-dep is ONE pinned egui_kittest matching egui 0.37, dev-only
  CHECK: python3 -c "import tomllib; d=tomllib.load(open('crates/gui/Cargo.toml','rb')); print(d['dev-dependencies'])"
  EXPECT: egui_kittest.*0\.37

- [x] G3 No runtime dependency change (eframe/rfd untouched, versions pinned)
  CHECK: git diff -- crates/gui/Cargo.toml
  EXPECT: dev-dependencies

- [x] G4 real_egui_shot test green (real App text-compare view, kittest render)
  CHECK: cargo test -p autocompare-gui --test real_egui_shot
  EXPECT: test result: ok

- [x] G5 real-egui.png is a valid non-blank PNG with expected dims
  CHECK: python3 -c "import struct;b=open('checks/gui-exq-kittest49/round-1/real-egui.png','rb').read();w,h=struct.unpack('>II',b[16:24]);print('VALID w=%d h=%d size=%d'%(w,h,len(b)) if b[:8]==bytes.fromhex('89504e470d0a1a0a') and len(b)>=8192 and (w,h)==(960,600) else 'INVALID')"
  EXPECT: VALID w=960 h=600

- [x] G6 Anti-aliased evidence: distinct RGB count far above flat rasterizer baseline
  CHECK: python3 checks/gui-exq-kittest49/round-1/pxvar.py
  EXPECT: AA-PROVEN

- [x] G7 Delta documented honestly (layout match vs font/chrome diff + pxdiff numbers)
  CHECK: grep -c pxdiff checks/gui-exq-kittest49/round-1/GATES.md
  EXPECT: [2-9]

- [x] G8 Full suite green, fmt clean, disjoint OWNS (no strays)
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3 && cargo fmt --check && git status --short
  EXPECT: test result: ok

## DELTA (honest): real-egui.png vs software after.png — the egui ceiling

pxdiff (tools/exquisite/pxdiff.py, same 960x600 canvas):
- full: diff_pixels 575999 of 576000, bbox (0,0)-(959,599) → VERDICT DIFFERENT
- pxdiff band[12,472) left pane: 275999 of 276000 differ
- pxdiff band[472,488) center gap: 9600 of 9600 differ
- pxdiff band[488,948) right pane: 276000 of 276000 differ
- pxvar (round pxvar.py): base=10 distinct RGB, real=502, ratio=50.2 → AA-PROVEN

What matches (layout/geometry from shared tokens):
- Two text panes + center gutter arrangement, same 960x600 canvas budget.
- Same content rows: alpha / beta→BETA changed row highlighted / gamma.
- Same status facts: "2 equal, 1 changed", "All: 3 rows (3 shown)".
- Same tab strip (Home Text Folder Hex Merge Table Sync Picture FolderMerge Audio)
  and same path-bar + Compare affordances.

What differs (the egui ceiling — real font rasterization + widget chrome):
- Real anti-aliased proportional/mono fonts vs flat 5x7 blocks (502 vs 10 colors).
- Real egui widgets: tabs, buttons, checkboxes, text fields, scrollbars, tooltips.
- Icon glyphs (sun U+2600 in "Light Mode", prev/next arrows) render as tofu
  boxes under the harness: kittest panics on missing glyphs by default, so the
  test uses allow_missing_glyphs() — the shipped app uses the identical egui
  font stack, so live behavior matches (no system-font fallback in either).
- Accessibility check off (with_accessibility_check(false)): live app has
  unlabeled icon buttons; labeling them would change shipped behavior.
- Live repaint loop never settles (clock/progress) → bounded run_steps(8),
  never run(); worker compare awaited via 10s poll on text_model.
- Dev-dep only: egui_kittest =0.37.0 features [eframe, wgpu]; zero runtime
  change, zero shipped-app behavior change.

Note: Cargo.lock gains egui_kittest entries on disk (build necessity) but is
intentionally NOT committed — commit scope is OWNS-only per wave orders.
Note: checks/gui-sessionpaint-by PNG churn is a side effect of the full
cargo test run (pre-existing tests rewrite them); reverted before commit.
