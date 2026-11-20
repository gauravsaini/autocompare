# REPORT — wave gutter28 (overview minimap fix, round-1)

Scope: overview minimap parity in fixture renderer against Beyond Compare reference.
BC displays a compressed miniature map in the right-hand strip where each file row maps
to a proportional vertical position inside a small inset. This wave repairs the unclosed delimiter
in `screenshot.rs`, connects `paint_overview_strip`, `overview_row_y`, and `overview_row_at_y` to
live padded geometry (`gutter_overview_geometry_padded`, `gutter_padded_row_at_y`, `gutter_padded_y_for_row`),
and preserves the center 20px lane with hunk-head markers.
Geometry FIRST, typography SECOND (no delta), colors FROZEN.

## Touched functions (all in `crates/gui/src/screenshot.rs`)

NEW (pure non-test helpers):
- `overview_row_y` — maps a file row to canvas y inside track inset using live `gutter_padded_y_for_row`.
- `overview_row_at_y` — inverse mapping from canvas y back to row using live `gutter_padded_row_at_y`.
- `paint_overview_strip` — renders full-file overview bands (same + diff) using `gutter_overview_geometry_padded` with `GUTTER_SEGMENT_MIN_H` floor and frozen colors.

CHANGED (non-test paint):
- `paint_bottom_legend_strip` — closed delimiter (added missing closing brace).
- `render_text_fixture_png` — uses `paint_overview_strip` for right overview minimap.
- `render_narrow_text_fixture_png` — uses `paint_overview_strip` for right overview minimap at 720px.

UNTOUCHED:
- `views.rs`, `theme.rs` — NEVER touched.
- Center 20px lane (`text_fixture_center_layout`, `text_center_gutter_width`, `TEXT_CENTER_MARK`).

TESTS (in `crates/gui/src/screenshot.rs`):
- `gutter28_overview_row_mapping`
- `gutter28_center_layout_intact`
- `gutter28_narrow_intact`
- `gutter28_overview_edge_fixture`

## Evidence

- Baseline: harness `RESULT: PASS`, before.png recorded.
- Post-change: harness `RESULT: PASS` (PNG 11059 B, magic ok, distinct 256).
- Suite: `cargo test -p autocompare-gui` -> 1058 passed / 0 failed.
- Rustfmt: clean (`rustfmt --edition 2024 --check crates/gui/src/screenshot.rs`).
- Banned-token grep on added lines: ZERO-BANNED (unwrap/expect/panic/todo/unreachable/unsafe/indexing).
- Edge fixture: `comparison/gutter28-overview-720.json` verified with headless test.
- Pixel gate (verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 3488 of 576000
diff_bbox: (936,178)-(951,555)
VERDICT: DIFFERENT
```
  Bbox honesty: x-range 936..951 covers the right overview strip (GUTTER_X=936, GUTTER_W=16); y-range 178..555 sits inside the pane height (174..560).
