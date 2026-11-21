# GATES: exq-geometry77 round-1 geometry polish (views.rs ONLY)

OWNS: crates/gui/src/views.rs, checks/gui-exq-geometry77/**

Scope: views.rs ONLY — 6 magic px literals alias existing SPACE-scale tokens
(same values, zero visual change). Colors FROZEN, typography FROZEN, AA intact.
Ponytail: reuse only, no new consts, no new hues, no size moves.

Reuse map (value-identical):
- FOOTER_GAP 6.0 -> crate::theme::GUTTER_BAR_NARROW (6.0 half-MD step)
- TAB_BOTTOM_GAP 6.0 -> crate::theme::GUTTER_BAR_NARROW
- TOOLBAR_GLYPH_W/H 16.0 -> crate::theme::SPACE_LG (16.0)
- SPLIT_VISUAL_W 1.0 -> crate::theme::TEXT_INLINE_SPAN_UNDERLINE_W (1.0)
- HEADER_H 28.0 -> crate::theme::CHROME_HEADER_BAR_H (28.0)
- New contract fn geometry77_space_scale_parity() locks all six + GUTTER_W==MD.

- [x] G1: Baseline exquisite harness PASS before edit
  CHECK: tools/exquisite/run.sh checks/gui-exq-geometry77/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: RESULT: PASS pre-edit (PNG 10946B, magic ok, distinct 256) EXIT 0.

- [x] G2: Magic literals gone, token aliases in place (views.rs non-test code)
  CHECK: cargo test -p autocompare-gui geometry77 -- --nocapture
  EXPECT: test result: ok
  EVIDENCE: views::tests::geometry77_space_scale ok (1 passed); 6 aliases value-identical, parity fn true, split/glyph insets bit-identical.

- [x] G3: Full gui suite green (lib + integration)
  CHECK: cargo test -p autocompare-gui
  EXPECT: test result: ok
  EVIDENCE: 1141 passed 0 failed (lib) + 2 passed real_egui_shot.

- [x] G4: fmt clean + banned tokens 0 on added diff lines
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: clean
  EVIDENCE: FMT_CLEAN after cargo fmt -p autocompare-gui; BANNED_0 on added diff lines (no unwrap/expect/panic/todo/unreachable/unsafe).

- [x] G5: Harness re-run PASS + kittest documented (no live-paint claim from after.png)
  CHECK: tools/exquisite/run.sh checks/gui-exq-geometry77/round-1 && cargo test -p autocompare-gui --test real_egui_shot
  EXPECT: RESULT: PASS
  EVIDENCE: RESULT: PASS post-edit (PNG 10946B identical size, colors touches 35 unchanged); real_egui_shot 2 passed (text_compare_shot + light). after.png is headless --screenshot artifact only — no live-paint claim.
