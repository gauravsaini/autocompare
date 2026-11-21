# REPORT — wave text38 (live center gutter geometry, round-1)

Scope: Exquisite text38 center gutter geometry against Beyond Compare reference
behavior (center 20px lane alignment, copy arrows centered, row height TypoRole Mono
rhythm, 1200px no h-scroll + narrow700 edge).
Geometry FIRST, typography SECOND, colors FROZEN at 38 touches.

## Touched (all owned)

- `crates/gui/src/gutter.rs`
  - Center 20px lane alignment: `TEXT38_CENTER_LANE_W` (20.0px = `SPACE_LG + SPACE_XS`),
    `text38_center_lane_layout`, `text38_center_lane_col_x`, `text38_center_lane_cx`,
    `text38_center_lane_is_aligned`. Perfect canvas midpoint lock across 1200px (cx=600),
    960px (cx=480), and 700px (cx=350).
  - Copy arrows centered: `TEXT38_COPY_ARROW_BOX` (16.0px = `SPACE_LG`), `TEXT38_COPY_ARROW_PAD_X`
    (2.0px = `GUTTER_MERGE_GAP`), `text38_copy_arrow_rect`, `text38_copy_arrow_in_row`,
    `text38_copy_arrow_hit_contains`. 16px box centered horizontally within 20px lane (2px per side)
    and vertically within row height.
  - Row height TypoRole Mono rhythm: `text38_mono_role` (`TypoRole::Mono`), `text38_mono_line_height`
    (1.30), `text38_mono_line_height_px` (15.6px), `text38_row_height_compact` (20.0px),
    `text38_row_height_padded` (19.6px), `text38_row_center_y`, `text38_row_top_y`, `text38_total_height`.
  - 1200px no h-scroll + narrow700 edge: `text38_canvas_w`, `text38_narrow_w`, `text38_avail_w`,
    `text38_narrow_avail_w`, `text38_needs_hscroll`, `text38_wide1200_fits`, `text38_narrow700_fits`.
  - Headless unit tests in `mod exq_text38_tests`: `text38_center_20px_lane_alignment`,
    `text38_copy_arrows_centered_in_lane`, `text38_row_height_typo_mono_rhythm`,
    `text38_wide1200_no_hscroll_and_narrow700_edge`, `text38_fixtures_parity`.
- `checks/gui-exq-text38/`
  - `round-1/GATES.md` (written BEFORE implementing per $unlazy)
  - `round-1/REPORT.md` (this report)
  - `round-1/after.png`, `round-1/before.png`, `round-1/exquisite-report.md`, `round-1/tests.out`
  - `round-1/comparison/text38-wide-1200.json`, `round-1/comparison/text38-narrow-700.json`
  - `comparison/text38-wide-1200.json`, `comparison/text38-narrow-700.json`

UNTOUCHED: all other gui files (`views.rs`, `copy.rs`, `take.rs`, `screenshot.rs`, `viewmodel.rs`, `theme.rs`), `crates/sessions/**`, `crates/compare/**`, `crates/diff/**`, `crates/vfs/**`.

## Evidence

- Baseline: harness `RESULT: PASS` (PNG 11040 B, magic ok, distinct 256).
- Post-change: harness `RESULT: PASS` (PNG 11040 B, magic ok, distinct 256).
- Pixel gate (verbatim):
```
before: 960x600 ch=4
after:  960x600 ch=4
diff_pixels: 0 of 576000
diff_bbox: none
VERDICT: IDENTICAL
```
- Full test suite: 1085 passed, 0 failed.
- Rustfmt: clean (`rustfmt --edition 2024 --check crates/gui/src/gutter.rs`).
- Clippy: clean (0 warnings on `gutter.rs`).
- Banned-token grep on added lines: ZERO-BANNED (unwrap, expect, panic, todo, unimplemented, unreachable, unsafe, non-test indexing).
- Colors frozen: diff adds 0 `Color32::from_rgb|Visuals::` (frozen at 38).
- Headless fixtures: `text38-wide-1200.json` and `text38-narrow-700.json` verified via `include_str!`.
