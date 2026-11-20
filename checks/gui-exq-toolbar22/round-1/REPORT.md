# REPORT — exq-toolbar22 round-1 (live egui toolbar glyph slots)

Base: 888db3c. Scope: `crates/gui/src/views.rs` ONLY (+ `checks/gui-exq-toolbar22/**`). No other paths touched.

## What changed (own words, clean-room: original geometric glyphs only)

The live session action bar keeps its button rects, labels, gaps and typo roles, and each button gains a fixed 12px abstract mark left of its label painted AFTER the Button response via `painter.rect_filled` in dim ink, so Button styling never overrides the ink (gutter23 precedent). Row-1 Swap/Reload rotate the fixture rhythm (solid block / twin bars by button index); row-2 Prev/Next carry the `<` / `>` chevron triples (five 2x2 cells, 6x10 span); the find-state bar Prev/Next carry the same chevrons. All paint uses existing tokens (`TIGHT_GAP` lead, `dim_for` ink); label text, min-sizes, gaps and colors untouched.

## Functions touched (all in `crates/gui/src/views.rs`)

- `TOOLBAR_GLYPH_W` / `TOOLBAR_GLYPH_H` consts + `toolbar_glyph_width()` / `toolbar_glyph_lead()` / `toolbar_glyph_variant()` / `toolbar_glyph_rects()` / `toolbar_glyph_slot()` (new pure geometry helpers, fixture `icon_box`/`icon_x_off`/`shown % 4`/`chevron_cells` parity) + `paint_toolbar_glyph()` (new painter overlay) — wired into `paint_toolbar_action_buttons` (Swap→v0, Reload→v1), `paint_action_nav_row` (Prev→v4, Next→v5) and `paint_text_find_state_bar` (Prev→v4, Next→v5). Labels, roles, rects, gaps unchanged.
- `toolbar22_live_glyph_slots` (new headless test) — proves the edge fixture.

No new helpers beyond these, no new dependencies, no `screenshot.rs`/`theme.rs`/`sessions` changes.

## Pixel gate (verbatim)

- Baseline self-compare: `before.png`/`after.png` byte IDENTICAL (11254 B each, `cmp` clean) — see ABANDONED note.
- Post-edit harness: `RESULT: PASS` (PNG 11254B, magic ok, distinct 256 ok; colors 33; geometry/typo signals info-only).

## Gates

- Harness: `RESULT: PASS` (full exquisite-report pasted below).
- Tests: `cargo test -p autocompare-gui` → 1047 passed, 0 failed.
- Edge fixture: `checks/gui-exq-toolbar22/round-1/comparison/toolbar-live-1200.json` + `toolbar22_live_glyph_slots` (1200px avail: band 24+8+24=56 stable, no h-scroll, 12px slots aligned, chevron 6x10 span, pair floor 200 / side air 500).
- `cargo fmt --check`: clean.
- Banned tokens on non-test delta (`unwrap(`/`expect(`/`panic!`/`todo!`/`unreachable!`/indexing): empty.
- Colors: exactly 33 (`Color32::from_rgb` + `Visuals::`), glyphs reuse `dim_for` only.
- Fixture pxdiff: ABANDONED with reason "egui-only paint change (`views.rs` overlay, `screenshot.rs` untouched); no fixture PNG delta expected" — verified IDENTICAL.
- Live-pixel gate: PASS (output below, also in GATES.md); zoomed crop `round-1/live-crop.png` (2638x397). Binary used: `/tmp/target-exq-toolbar22/debug/autocompare-gui` (same binary the harness builds via `CARGO_TARGET_DIR`; avoids a duplicate build).

## Exquisite report (post-edit, verbatim)

```
# Exquisite report (round-1)
- PNG: 11254 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG', 'MD', 'SM', 'XL', 'XS'], margin/gap consts x611, Grid x16, row-h x327, width calls x85, gutter x1753
- TYPO: TypoRole x1063, monospace x26, strong x45, .size ['14.0'], weights ['TYPO_WEIGHT_MEDIUM', 'TYPO_WEIGHT_REGULAR', 'TYPO_WEIGHT_SEMIBOLD'], line-heights ['TYPO_LINE_HEIGHT_BODY', 'TYPO_LINE_HEIGHT_CHROME', 'TYPO_LINE_HEIGHT_MONO', 'TYPO_LINE_HEIGHT_SOURCE'], spacing ['TYPO_LETTER_SPACING', 'extra_letter_spacing']
- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)
RESULT: PASS
```

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

Gate passes: (a) 6/6 ≥ 6, ink/face delta 100 ≥ 40; (b) 2 ≥ 2. Script: `round-1/live-pixel-check.py` (run via `uv run --with pillow`).
