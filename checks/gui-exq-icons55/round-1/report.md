# Report — exq-icons55 round-1: staged vector layer live wiring in app.rs

Verdict: **WIRED** (live action bar: Swap and Reload execute staged vector mark paint live via `App::paint_session_action_bar_live`, moving 332px in `real-egui.png`; other sites honestly evaluated).

## Wired-vs-not table

| Live site (`app.rs` call site) | Staged fn | Wired? | Reason + Pixel Proof |
|---|---|---|---|
| Toolbar row 1: `App::paint_session_action_bar_live` -> `views::paint_session_action_bar` -> `paint_toolbar_action_buttons` (Swap / Reload) | `paint_exq_icons50_mark` (variants 0 & 1 -> `Ring` & `Bars`) | **YES (WIRED)** | Live delegated via `App::paint_session_action_bar_live` into `views::paint_toolbar_glyph`. Uses exact button rect + `exq_icons50_mark_xy` origin + `dim_for` ink. **Pixel proof**: 332 pixels moved in `real-egui.png` inside bounding box `x=[16, 135], y=[113, 128]`. Unit test `toolbar_exq_icons55_live_vector_wiring` proves `Shape::Circle` (radius 4.0) and `Shape::LineSegment` marks are painted into egui painter. Hit rects and click interactions remain 100% byte-identical. |
| Toolbar row 2: `paint_action_nav_row` (Prev / Next jump cluster) | `paint_exq_icons50_mark` | **NO** | Navigation affordance requires directional chevrons `<` / `>` (variants 4 & 5). `paint_exq_icons50_mark` only defines Ring, Bars, Rails, Wedge without any chevron marks. Routing to vector marks would corrupt the `<` / `>` arrow glyphs. Chevron rects preserved honestly. |
| Mode tabs: `views::paint_tabs` (10 `selectable_value`) | `paint_exq_icons50_tab_pill` | **NO** | Text occlusion risk: calling `painter.rect_filled` after `ui.selectable_value` paints an opaque fill over the text label ("Home", "Text"), hiding the text. Replacing `selectable_value` with custom allocate would risk breaking tab selection and interaction semantics. |
| Session strip: `views::paint_session_tab_strip` (`paint_session_tab`) | `paint_exq_icons50_tab_pill` | **NO** | Same text occlusion risk as mode tabs (`Button::selectable` paints text first). Additionally, in default compare view (`real_egui_shot`), session list is empty, rendering the empty hint label instead of session tab pills. |
| 10-action TOPCHROME toolbar (`TOPCHROME_TOOLBAR_ACTIONS`) | `paint_exq_icons50_mark` (kinds 0–9) | **N/A** | Fixture-only toolbar: exists in `screenshot.rs` pixels + `home.rs` geometry, has no live egui counterpart (live bar = 4 buttons only). Nothing to wire to in live `app.rs`. |

## Pixel evidence

### 1. Headless test comparison (`tests/real_egui_shot.rs`)
- Baseline `real-egui.png` sha256: `ba868694ffc2c1eb9917adc63256ce1320bdf7ae1fbcb12b14e814c3a6eaa540` (size: 52,575 bytes, colors: 502)
- Post-change `real-egui.png` sha256: `b156e8cb1f6e4068c835b67f723bba6f55657839c66d415730f43158b615e0fb` (size: 52,782 bytes, colors: 502)
- Pixel delta analysis:
  - Total differing pixels: **332**
  - Bounding box of diff: **x=[16, 135], y=[113, 128]** (exact height 16px, exact width covering Swap and Reload toolbar glyph slots).
  - Collateral drift: 0 pixels outside the toolbar button slot.

### 2. Harness screenshot comparison (`after.png`)
- Baseline `after.png` sha256: `ff5c187d773fc7b41831c4708dd5bd7daaf248c45780a9af5b3e609b698a2700` (10,973 bytes)
- Post-change `after.png` sha256: `ff5c187d773fc7b41831c4708dd5bd7daaf248c45780a9af5b3e609b698a2700` (10,973 bytes)
- Note: Harness `--screenshot` bypasses `App` egui paint and calls `screenshot::render_fixture_png_with_mode` directly, so `after.png` is identical as expected. Live pixel proof is fully verified by `real_egui_shot.rs`.

## Quality & Contract Compliance
- Zero new `Color32` tokens: reused existing `dim_for(ui.visuals().dark_mode)`.
- Zero new layout/size tokens: reused `exq_icons50_mark_xy`, `SPACE_XS`, `SPACE_MD`.
- Banned call scan: `0` matches for `Color32::from_rgb`, `Visuals::`, `unsafe`, `unwrap()`, `expect(`, `panic!`, `todo!`.
- Full cargo test suite: 1113 unit tests + 1 integration test passing green.
