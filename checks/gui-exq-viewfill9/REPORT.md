# VIEWFILL9 Report — tall 1200px plus dual panes and gutter column (round-1)

Wave: exq-viewfill9 leaf. Geometry FIRST, typography SECOND, colors FROZEN at 33 touches.

## Kya mila (gap, apne shabdon me)

- Beyond Compare text compare view parity gap: tall 1200px canvas height par dual text-compare panes ki scroll canvas height budget (`1200.0 - header/file-bar chrome = 1156.8px`), side-by-side vs over-under (stacked vertical) pane heights (`1156.8px` side-by-side equal, `574.4px` over-under equal), tall canvas par file-bar rhythm lock (`15.2px` thin chrome, stretch/drift nahi karta), aur center gutter column vertical spans over tall canvas (1156.8px scroll canvas height, vertical arrow centering at canvas midpoint 600.0px / scroll midpoint 578.4px, symmetric 2px hit slop, segment vertical tracking `text_tall_segment_arrow_cy`) missing the.
- Tall-1200 text-side edge fixture `views-tall-1200.json` missing tha to prove tall canvas height budget, file-bar lock, dual pane equal heights, and center gutter geometry.

## Kya badla (sirf OWNS files)

`crates/gui/src/views.rs`:
1. **Tall 1200px canvas height constants and budgets**:
   - `TEXT_TALL_CANVAS_H`: 1200.0px.
   - `text_view_header_chrome_h()`: returns standard header/toolbar bar height (`HEADER_H` = 28.0px).
   - `text_chrome_height_budget()`: returns total header toolbar plus thin file-bar chrome (`28.0 + 15.2 = 43.2px`).
   - `text_tall_scroll_canvas_height(avail_h)`: returns remaining scroll canvas height budget after chrome deduction (`avail_h - 43.2px`, floored at `TEXT_PANES_MIN_H`).
   - `text_tall_1200_scroll_canvas_height_px()`: yields exactly 1156.8px (`1200.0 - 43.2`).
   - `text_tall_1200_pane_height_px()`: yields 1156.8px (side-by-side pane height).
2. **Dual-pane equal heights (side-by-side vs over-under)**:
   - `text_side_by_side_pane_heights(avail_h)`: returns equal pair `(h, h)` where `h = text_tall_scroll_canvas_height(avail_h)`. At 1200px: `(1156.8, 1156.8)`.
   - `text_tall_1200_side_by_side_pane_heights()`: yields `(1156.8, 1156.8)`.
   - `text_overunder_pane_heights(avail_h)`: returns stacked vertical equal pair `(h, h)` where `h = (scroll_h - SECTION_GAP) / 2.0`. At 1200px: `(574.4, 574.4)`.
   - `text_tall_1200_overunder_pane_heights()`: yields `(574.4, 574.4)`.
3. **File-bar rhythm locked over tall canvas**:
   - `text_file_bar_height_for_canvas(canvas_h)`: returns locked 15.2px (`text_file_bar_height_px()`) across any canvas height (never stretches or drifts).
   - `text_tall_1200_file_bar_height_px()`: yields locked 15.2px.
4. **Center gutter column over tall canvas**:
   - `text_tall_1200_center_gutter_height_px()`: runs full height of scroll canvas (1156.8px).
   - `text_tall_canvas_center_y(avail_h)`: vertical canvas middle (`avail_h / 2.0` = 600.0px at 1200px).
   - `text_tall_scroll_canvas_center_y(avail_h)`: vertical scroll canvas middle (`1156.8 / 2.0 = 578.4px`).
   - `text_center_hit_band_for_avail(avail_w)`: hit band with 2px slop (`[348.0, 372.0]` at 720px, `[468.0, 492.0]` at 960px).
   - `text_center_arrow_cx_for_avail(avail_w)`: center marker horizontal cx (`360.0` at 720px, `480.0` at 960px).
   - `text_tall_segment_arrow_cy(row_start, row_end, row_h)`: vertical midpoint tracking for hunk/diff segments.
   - `text_tall_hunk_arrow_cy(row_idx, row_h)`: vertical centering inside row beat (`row_idx * row_h + row_h / 2.0`).
5. **Headless test**:
   - `viewfill9_tall_1200_dual_panes_plus_gutter_column`: Proves edge fixtures `views-tall-1200.json` and `views-narrow-720.json`, dual-pane heights (side-by-side 1156.8 and over-under 574.4), file-bar lock (15.2px), center gutter column geometry, hit band slops, arrow vertical centering (600.0 / 578.4), segment tracking, and edge guards.
6. **Zero banned constructs**:
   - Non-test code me `unsafe`, `unwrap()`, `expect(`, `panic!`, `todo!`, `unreachable!` aur slice indexing 0.
7. **Colors FROZEN**:
   - `Color32::from_rgb` / `Visuals::` touch count 33 strictly preserved.

## Edge fixtures

`checks/gui-exq-viewfill9/comparison/views-tall-1200.json` aur `checks/gui-exq-viewfill9/round-1/comparison/views-tall-1200.json`:
- `avail_h`: 1200.0
- `avail_w`: 720.0
- `avail_w_wide`: 960.0
- `header_chrome_h`: 28.0
- `file_bar_h`: 15.2
- `chrome_budget_h`: 43.2
- `scroll_canvas_h`: 1156.8
- `pane_h`: 1156.8
- `overunder_pane_h`: 574.4
- `center_gutter_w`: 20.0
- `center_gutter_h`: 1156.8
- `column_x_720`: 350.0
- `column_x_960`: 470.0
- `arrow_cx_720`: 360.0
- `arrow_cx_960`: 480.0
- `canvas_middle_cy`: 600.0
- `scroll_canvas_middle_cy`: 578.4
- `hit_band_720`: [348.0, 372.0]
- `hit_band_960`: [468.0, 492.0]
- `file_bar_locked_tall`: true
- `segment_tracking_middle_row_34_cy`: 607.2

## Natija

- Headless test `viewfill9_tall_1200_dual_panes_plus_gutter_column` PASS.
- Full lib test suite (1019 tests) PASS.
- Exquisite harness `tools/exquisite/run.sh checks/gui-exq-viewfill9/round-1 crates/gui/src`: RESULT: PASS.
- Colors frozen at 33 touches.
