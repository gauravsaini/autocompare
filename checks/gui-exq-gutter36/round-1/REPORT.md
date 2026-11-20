# REPORT — wave gutter36 (overview strip polish, round-1)

Scope: fixture-only polish of the right overview minimap against the
reference behavior (whole file compressed into a thin right strip, each
row at a proportional height inside a small top/bottom inset).
Geometry FIRST, typography SECOND (no delta), colors FROZEN.

## Touched (all owned)

- `crates/gui/src/screenshot.rs`
  - `paint_overview_strip`: band edges now `.round()` (was truncation via
    `as u32`), matching `overview_row_y` which rounds the same live
    mapping — max 1px paint-vs-marker drift gone. `GUTTER_SEGMENT_MIN_H`
    floor and frozen hues unchanged. Single source for wide (960) and
    narrow (720) routes.
- `checks/gui-exq-gutter36/round-1/`
  - `comparison/gutter36-overview-polish.json` (edge fixture: pad 4,
    inner 378 @386h, first/last/mid row y, narrow 720 columns, rounding=round)
  - headless tests in `screenshot.rs`: `gutter36_overview_alignment`,
    `gutter36_narrow_intact`, `gutter36_overview_edge_fixture`.

UNTOUCHED: `views.rs`, `theme.rs`, `gutter.rs`, center 20px lane
(`text_fixture_center_layout` = (12, 448, 464, 20, 488, 448)).

## Evidence

- Baseline: harness `RESULT: PASS` (PNG 11059 B), `before.png` recorded.
- Post-change: harness `RESULT: PASS` (PNG 11040 B, magic ok, distinct 256).
- Suite: `cargo test -p autocompare-gui` -> 1068 passed / 0 failed.
- Rustfmt: clean (`rustfmt --edition 2024 --check crates/gui/src/screenshot.rs`).
- Banned-token grep on added lines: ZERO-BANNED (unwrap/expect/panic/todo/unreachable/unsafe).
- Colors: diff adds 0 `Color32::from_rgb|Visuals::`.
- Narrow headless: `/tmp/gutter36-narrow-check.png` 720x600 magic ok distinct 256.
- Pixel gate (verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 32 of 576000
diff_bbox: (936, 339)-(952, 448)
VERDICT: DIFFERENT
```
  Bbox honesty: x-range 936..952 is exactly the overview strip
  (GUTTER_X=936, GUTTER_W=16); y-range 339..448 sits inside pane
  174..560 — rounding-only band-edge shift, nothing else moved.
