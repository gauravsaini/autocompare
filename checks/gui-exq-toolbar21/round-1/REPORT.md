# REPORT — exq-toolbar21 round-1 (fixture toolbar icon+label row)

Base: dd94ce9. Scope: `crates/gui/src/screenshot.rs` ONLY (+ `checks/gui-exq-toolbar21/**`). No other paths touched.

## What changed (own words, clean-room: original geometric glyphs only)
Row-1 toolbar buttons keep their rhythm rects, labels, and spacing, and each gains a 12px abstract mark left of its label (solid block / twin bars / side bars / stepped wedge, rotating by button index — simple marks, not copies of any reference artwork). Row-2 Prev/Next buttons keep their 96x24 rects, labels, and 8px cluster air, and each gains a small original `<` / `>` triple built from three `fill_rect` bars. All paint uses the existing button-rhythm helpers and the existing palette hues; pane_top (174), center gutter, hunk rows, and bottom strips are untouched.

## Functions touched (all in `crates/gui/src/screenshot.rs`)
- `paint_topchrome_toolbar` — row-1 icon glyph boxes + label shift/clip (`vw-24` budget); row-2 chevron triples + label shift. Only non-test function changed.
- `toolbar21_narrow720_no_overlap` (new headless test) — proves the narrow-720 edge fixture.

No new helpers, no new dependencies, no `views.rs` changes.

## Pixel gate (verbatim)
- Baseline self-compare: `before: 960x600 ch=4 / after: 960x600 ch=4 / diff_pixels: 0 of 576000 / diff_bbox: none / VERDICT: IDENTICAL` (exit 0).
- Post-edit: `before: 960x600 ch=4 / after: 960x600 ch=4 / diff_pixels: 1995 of 576000 / diff_bbox: (16,120)-(866,162) / VERDICT: DIFFERENT` (exit 0, `--expect-diff`).
- Intactness: the diff sits inside the toolbar band (y114..170); all pixels outside `(16,120)-(866,162)` are byte-identical, so tab chrome (y~40), center gutter marks (x~464..484), hunk rows, per-pane strips (y542..560), and bottom strips (y570..600) are unchanged.

## Gates
- Harness: `RESULT: PASS` (PNG 11254B, magic ok, distinct 256 ok).
- Tests: `cargo test -p autocompare-gui` → 1044 passed, 0 failed.
- Edge fixture: `checks/gui-exq-toolbar21/round-1/comparison/toolbar-narrow-720.json` + `toolbar21_narrow720_no_overlap` (720px: 7 visible buttons tile gap-exact with icons+clipped labels, no overlap; nav pair 12/116 + chevrons fit; center layout `(12,448,464,20,488,448)` intact).
- `cargo fmt --check`: clean.
- Banned tokens on non-test delta (`unwrap(`/`expect(`/`panic!`/`todo!`/`unreachable!`/indexing): empty.
- Colors: exactly 33 (`Color32::from_rgb` + `Visuals::`), glyphs reuse existing palette only.
