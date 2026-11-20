# GATES toolbar22 (written BEFORE implementing per $unlazy)

## CHECK: baseline harness runs on fresh round-1 dir
- CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-toolbar22 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-toolbar22/round-1 crates/gui/src` exits 0 and writes `after.png` + `exquisite-report.md` with `RESULT: PASS`; then `cp round-1/after.png round-1/before.png`.
- EXPECT: `after.png` valid PNG >= 8192B non-blank; report `RESULT: PASS`; `before.png` byte-identical to `after.png`.

## CHECK: glyph slots mirror fixture geometry (fill_rect only, existing palette)
- CHECK: in `crates/gui/src/views.rs` ONLY: add `TOOLBAR_GLYPH_W` (12.0) const + pure helpers `toolbar_glyph_width()`, `toolbar_glyph_variant(index)` (`index % 4`, fixture `shown % 4` rhythm), `toolbar_glyph_rects(variant)` returning slot-local `(dx,dy,w,h)` rects for variants 0..=5 (0 solid 12x12, 1 twin bars, 2 side bars, 3 stepped wedge, 4 prev `<` chevron triple, 5 next `>` chevron triple) + one `paint_toolbar_glyph(ui, slot, variant)` painter helper (`ui.painter().rect_filled`, ink `dim_for(dark)` — fixture dim-hue reuse, no new `Color32`). Overlay AFTER each `egui::Button` (painted on top so Button styling cannot override, gutter23 precedent): row-1 Swap→v0 / Reload→v1 at `rect.left()+4`, vertically centered 12px box; row-2 Prev→v4 / Next→v5; find-state bar Prev→v4 / Next→v5 (6 buttons total). Labels/typo (`toolbar_swap_label`, `action_nav_prev_label`, roles) UNCHANGED; button rects/min-sizes/gaps UNCHANGED (overlay, zero layout shift).
- EXPECT: full suite green; `cargo fmt --check` clean; banned-token grep on non-test delta empty; colors exactly 33 (no new `Color32::from_rgb`/`Visuals::`).

## CHECK: exquisite re-run PASS + fixture pxdiff ABANDONED with reason
- CHECK: re-run same harness command into same round dir; paste full `exquisite-report.md` output into REPORT.md.
- EXPECT: `RESULT: PASS`; fixture pxdiff IDENTICAL (egui-only change, `screenshot.rs` untouched, no fixture repaint) — mark ABANDONED with reason "egui-only paint change; no PNG delta expected".

## CHECK: live-pixel gate (MANDATORY, gutter23 precedent)
- CHECK: build, launch `./target/debug/autocompare-gui --open tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --tab text`, sleep 6, `screencapture -x -o round-1/live.png`, kill GUI; then `uv run --with pillow` script crops the toolbar band (action-bar rows + find bar, anchored on the `Swap Sides`/`Reload`/`Prev`/`Next` button row geometry) and ASSERTs (a) glyph-slot pixels differ from button-face fill in at least 6 toolbar buttons (dim ink ~150 vs face ~60), (b) chevron pixels present in the nav row (2x2-bar `<`/`>` triples at Prev/Next left slots).
- EXPECT: both asserts hold; script output pasted into GATES.md + REPORT.md; zoomed crop saved as `round-1/live-crop.png`; gate passes only if both hold.

## CHECK: edge fixture + headless test
- CHECK: add `round-1/comparison/toolbar-live-1200.json` (tall/scroll: 1200px avail — toolbar band stays 24+8+24=56, glyph slots 12px left-of-label aligned on all 6 buttons, chevron triples in nav row, no overlap, center gutter layout intact) + one headless test named `toolbar22_live_glyph_slots` proving it.
- EXPECT: test green; fixture `include_str!` path correct; glyph/chevron rect math mirrors fixture `shown % 4` + chevron-cell precedent.

## CHECK: MS Rust + touch/scope + colors
- CHECK: `#![forbid(unsafe_code)]` holds; no `unwrap/expect/panic/todo/unreachable/indexing` in non-test delta; GUI stays thin client; `cargo fmt` run on touched files; `git status` shows ONLY `crates/gui/src/views.rs` + `checks/gui-exq-toolbar22/**`; NEVER touch `crates/sessions/*`, `crates/gui/src/screenshot.rs`, `crates/gui/src/theme.rs`; never `git add -A`; colors `Color32::from_rgb|Visuals::` == 33.
- EXPECT: full `autocompare-gui` suite green; `cargo fmt --check` clean; banned-token grep empty; colors 33.

## LIVE-PIXEL GATE OUTPUT (round-1, pasted verbatim)

```
live.png: 2880x1800
blue anchor: n=3732 x in [504,752] median xc=642
scan from x0=242
button-row bands: [(278, 286), (312, 316), (464, 478), (502, 510), (534, 542), (570, 580), (662, 740), (762, 770)]
solid block: x=[512,534] y=[476,498]
action row1 y=(464, 478), nav row2 y=(502, 510)
row1 runs: [(505, 695), (713, 1009)]
row1 center y1c=487 (from solid block)
nav center y2c=551 (face rows: 9 hits in +-20)
row2 runs: [(511, 688), (719, 897)]
find row y~741: (1205, 1280) (1283, 1375)
  Swap: bx0=505 yc=487 slots=1 [(512, 476, 24, 24, 144)]
  Reload: bx0=713 yc=487 slots=2 [(720, 494, 24, 6, 36), (720, 476, 24, 6, 36)]
  Prev: bx0=511 yc=551 slots=1 [(520, 548, 12, 20, 20)]
  Next: bx0=719 yc=551 slots=1 [(724, 548, 12, 20, 20)]
  FindPrev: bx0=1205 yc=741 slots=1 [(1206, 750, 4, 16, 14)]
  FindNext: bx0=1283 yc=741 slots=2 [(1302, 752, 4, 8, 7), (1308, 750, 4, 12, 11)]
gate (a): glyph slots on 6/6 toolbar buttons (ink ~150 vs face ~50, delta >= 40)
gate (b): chevron triples in nav row: 2
  chev x=[520,530] y=[548,566] w=12 h=20 pts=20
  chev x=[724,734] y=[548,566] w=12 h=20 pts=20
crop saved: checks/gui-exq-toolbar22/round-1/live-crop.png 2638x397
LIVE-PIXEL GATE: PASS
```

Gate passes: (a) 6/6 ≥ 6, ink/face delta 100 ≥ 40; (b) 2 ≥ 2. Script:
`round-1/live-pixel-check.py` (run via `uv run --with pillow`).
Zoomed crop: `round-1/live-crop.png` (2638x397).
