# Report — exq-geo55 round-1: Gutter Geometry Parity across Canvas Budgets

Verdict: **PASS** (Pure gutter geometry parity tightened for narrow <720px/700px, normal 960px, and wide 1200px canvases in `crates/gui/src/gutter.rs` ONLY).

## Summary of Changes

1. **Canvas Budget Allocator**:
   - Added `GutterCanvasBudget` struct and pure calculation `gutter_compute_canvas_budget` allocating exact pixel dimensions across outer margins, line-number columns, center divider/lane, and twin text panes without clipping (`pane_w > 0`) or double-padding (`total_allocated_w == canvas_w`).
   - Dedicated single-source budget helpers:
     - `gutter_budget_narrow_700`: 700px canvas (12 margin + 16 linenum + 312 pane + 20 center + 312 pane + 16 linenum + 12 margin = 700; or 328px panes without line numbers).
     - `gutter_budget_normal_960`: 960px canvas (12 margin + 16 linenum + 442 pane + 20 center + 442 pane + 16 linenum + 12 margin = 960; or 458px panes without line numbers).
     - `gutter_budget_wide_1200`: 1200px canvas (12 margin + 16 linenum + 562 pane + 20 center + 562 pane + 16 linenum + 12 margin = 1200; or 578px panes without line numbers).

2. **Single-Source Delegation**:
   - All spatial calculations delegate to existing `crate::theme` tokens:
     - Outer margin: `GEO_OUTER_MARGIN` (12.0)
     - Scale steps: `SPACE_XS` (4.0), `SPACE_SM` (8.0), `SPACE_MD` (12.0), `SPACE_LG` (16.0), `SPACE_XL` (24.0)
     - Gap / pad: `GUTTER_MERGE_GAP` (2.0)
   - Zero new tokens, zero new colors, zero typography edits.

3. **Alignment & Centering**:
   - Right-aligned line numbers: `gutter_linenum_right_aligned_x` using single-source pad.
   - Centered markers: `gutter_mark_centered_cx`, `gutter_mark_centered_cy`, and `gutter_center_mark_rect`.
   - Midpoint alignment validation: `gutter_mark_is_canvas_midpoint_aligned` ensures `cx == canvas_w / 2.0` holds across all canvas sizes.

4. **Omit Marker Affordance**:
   - Omit gutter column width: `GUTTER_OMIT_MARKER_COL_W` (16.0 = `SPACE_LG`).
   - Omit marker box: `GUTTER_OMIT_MARKER_BOX` (12.0 = `SPACE_MD`).
   - Padding & hit-testing: `gutter_omit_marker_pad` (2.0 = `GUTTER_MERGE_GAP`), `gutter_omit_marker_rect`, and `gutter_omit_marker_hit_contains` with 2.0px hit-slop.

5. **No Double-Padding & No Clipping**:
   - `gutter_verify_no_double_padding` and `gutter_budget_has_no_clipping_or_overflow` guarantee deterministic layout boundaries.

6. **Edge Fixture**:
   - Added `checks/gui-exq-geo55/round-1/comparison/gutter-narrow-700.json` and verified with headless test `geo55_narrow_700_edge_fixture`.

## Exquisite Signals Delta

| Signal | Baseline (round-1) | Post-Change (round-1) | Delta | Notes |
|---|---|---|---|---|
| PNG bytes | 10973 | 10973 | 0 | Harness screenshot valid |
| Distinct bytes | 256 | 256 | 0 | Non-blank |
| `gutter` count | 1823 | 1942 | +119 | Pure helpers added |
| `margin/gap` count | 706 | 717 | +11 | Single-source delegations |
| `TypoRole` count | 1145 | 1145 | 0 | Typography untouched |
| `Color32/Visuals` touches | 35 | 35 | 0 | Colors strictly frozen |
| Harness result | PASS | PASS | — | Green |

## Quality & Compliance

- Rust guidelines: `#![forbid(unsafe_code)]` preserved; 0 `unsafe`.
- Non-test banned calls: 0 matches for `unwrap()`, `expect(`, `panic!`, `todo!`, `Color32::from_rgb`, `Visuals::`.
- Unit tests: full suite passing (`1112 passed; 0 failed`).
- Formatting: `cargo fmt -p autocompare-gui -- --check` clean.
- Scope: touched ONLY `crates/gui/src/gutter.rs` and `checks/gui-exq-geo55/**`.
