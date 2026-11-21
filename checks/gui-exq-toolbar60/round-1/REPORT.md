# REPORT exq-toolbar60 round-1 — nav chevrons

## What changed (`crates/gui/src/views.rs` only)
- Row-2 `paint_action_nav_row` Prev/Next now paint stroke-vector chevrons
  (`<`/`>`) via new `paint_nav_chevron_mark` + pure `nav_chevron_segments`
  + `NavChevronDir`, reusing staged tokens only: `app::exq_icons50_mark_xy`
  origin (left lead 4 + vertical center), `app::exq_icons50_mark_box` 12px
  box, `app::exq_icons50_stroke_w` 2px stroke, `dim_for` ink — same
  paint-after-response pattern as row-1 Ring/Bars. Labels (`◀ Prev` /
  `Next ▶`), 96x24 floors, gaps, click behavior: byte-identical (overlay,
  zero layout shift).
- `paint_toolbar_glyph` variants 4/5 cell path untouched (find-bar
  Prev/Next at views.rs:9066/9071 intact); `toolbar_glyph_rects(4/5)`
  geometry tests untouched.
- Honest discrepancy vs brief: `app.rs` `ExqIcons50Mark` stages only
  Ring/Bars/Rails/Wedge (no chevron kinds) and `app.rs` is outside this
  leaf's OWNS, so the chevron vector layer lives in `views.rs` on staged
  geometry tokens. No new sizes/colors/tokens.

## Gates evidence
- G1 baseline: harness `RESULT: PASS` on fresh dir (before.png n/a —
  fresh round, no prior PNG to diff).
- G2: `git diff --stat` = `views.rs` only (+ this checks dir untracked);
  labels/floors untouched (unit test asserts `◀ Prev`/`Next ▶`, 96x24,
  pair 200).
- G3: `cargo test -p autocompare-gui toolbar60` →
  `toolbar_exq_toolbar60_nav_chevron_marks ... ok` (1 passed).
- G4: full suite `1122 passed lib + 2 kittest`, 0 failed;
  `cargo fmt --check` clean; banned-token grep on `+` delta empty.
- G5: harness re-run `RESULT: PASS` (PNG 10946B, magic ok, distinct 256);
  colors count 35 unchanged (no new Color32/Visuals).
- G6: commit scope verified below; no push.

## Kittest render (documented, `real_egui_shot`)
- `real-egui(real-egui.png): rows=3 bytes=53541 colors=503 base=10 ratio=50`
- `real-egui(real-egui-light.png): rows=3 bytes=54205 colors=549 base=10 ratio=54`
- Canvas dims: 960x600 (test-asserted `SHOT_W`/`SHOT_H` inside the test).
- Toolbar pixel delta box (geometry, per nav button): 12x12 mark box at
  `(button.left()+4, button.center.y-6)` — row-1 staged origin
  (`104,56` for a `(100,50,96x24)` button, unit-asserted); inside it the
  chevron strokes Prev tip-x=3/end-x=9, Next mirrored (9/3), y at
  2.4/6.0/9.6 of the box, 2px stroke, `dim_for` ink. Band: 24+8+24=56,
  row-2 pair floor 200. Shots stay in target tmp (tracked kittest58
  copies untouched — outside OWNS).

## Exquisite re-run report (verbatim tail)
- PNG: 10946 bytes (min 8192: ok), magic ok, distinct 256 (ok)
- GEOMETRY: SPACE tokens ['LG','MD','SM','XL','XS'], margin/gap x717,
  Grid x16, row-h x374, width x85, gutter x1979
- TYPO: TypoRole x1146, monospace x26, strong x45, .size ['14.0']
- COLORS: 35 (unchanged) — RESULT: PASS
