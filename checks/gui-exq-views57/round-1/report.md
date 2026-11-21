# Report — exq-views57 round-1: live tabs/session-strip vector polish in views.rs

Verdict: **WIRED** (live mode tabs and session tab strip: active tab adopts staged vector tab pill geometry via underline-only accent natively in `views.rs`, moving pixels in `real-egui.png` without text occlusion risk).

## Wired-vs-not table

| Live site (`views.rs` paint call site) | Staged fn | Wired? | Reason + Pixel Proof |
|---|---|---|---|
| Mode tabs: `views::paint_tabs` / `views::paint_tabs_returning_rects` (10 `selectable_value`) | `tab_active_underline_rect` adopting staged `exq_icons50_tab_pill_rect` + `exq_icons50_inline_accent_for` | **YES (WIRED)** | Adopted natively in `views::paint_tabs_returning_rects` called by `paint_tabs`. In live `real_egui_shot`, active tab is `Text`. Underline indicator is painted across `tab_active_underline_rect` at the base of the tab pill without text occlusion (indicator sits at `pill.max.y - 2.0 .. pill.max.y`). **Pixel proof**: 16 pixels moved in `real-egui.png` inside bounding box `x=[0, 140], y=[67, 98]`. Hit rects and tab switching interactions remain 100% byte-identical. |
| Session strip: `views::paint_session_tab_strip` (`paint_session_tab`) | `tab_active_underline_rect` adopting staged `exq_icons50_tab_pill_rect` + `exq_icons50_inline_accent_for` | **YES (WIRED)** | When `is_active` is true, `paint_session_tab` paints the active tab indicator. In default compare view (`real_egui_shot`), session list is empty, rendering the empty hint label. Unit test `tabs_exq_views57_live_vector_wiring` proves active session tab paints indicator. Hit rects and switch/close interactions remain 100% byte-identical. |
| Toolbar row 1: `views::paint_toolbar_action_buttons` (Swap / Reload) | `paint_exq_icons50_mark` (variants 0 & 1 -> `Ring` & `Bars`) | **PREVIOUSLY WIRED (views53)** | Preserved untouched in `views::paint_toolbar_glyph`. |
| Toolbar row 2: `views::paint_action_nav_row` (Prev / Next jump cluster) | `paint_exq_icons50_mark` | **NO** | Navigation affordance requires directional chevrons `<` / `>` (variants 4 & 5). Chevron rects preserved honestly. |
| 10-action TOPCHROME toolbar (`TOPCHROME_TOOLBAR_ACTIONS`) | `paint_exq_icons50_mark` (kinds 0–9) | **N/A** | Fixture-only toolbar in `screenshot.rs`. |

## Pixel evidence

### 1. Headless test comparison (`tests/real_egui_shot.rs`)
- Baseline `real-egui.png` sha256: `c0223a0d1d00cdbe1bb67cff3f29e28d71751dec1835e923a26c997d1921585a` (size: 52,672 bytes, colors: 502)
- Post-change `real-egui.png` sha256: `6643f596cfc6631df6098f3c89531ee9db7237386e52c8777ec17d161954257d` (size: 52,700 bytes, colors: 503)
- Pixel delta analysis:
  - Differing pixels: **16**
  - Bounding box of diff: **x=[0, 140], y=[67, 98]** (exact mode tabs chrome row area).
  - Collateral drift: 0 pixels outside the tabs band.

### 2. Harness screenshot comparison (`after.png`)
- Baseline `after.png` sha256: `a91bf97d39d91f2c97486e1fb20a0ee44754a02d4fbcfd18bce6d7a46f9ec674` (10,946 bytes)
- Post-change `after.png` sha256: `a91bf97d39d91f2c97486e1fb20a0ee44754a02d4fbcfd18bce6d7a46f9ec674` (10,946 bytes)
- Note: Harness `--screenshot` bypasses `App` egui paint and calls `screenshot::render_fixture_png_with_mode` directly, so `after.png` is identical as expected. Live pixel proof is fully verified by `real_egui_shot.rs`.

## Quality & Contract Compliance
- Zero new `Color32` tokens: reused `crate::app::exq_icons50_inline_accent_for(ui.visuals().dark_mode)`.
- Zero new layout/size tokens: reused `crate::app::exq_icons50_tab_pill_rect`, `SPACE_XS`, `TYPO_WEIGHT_SEMIBOLD`.
- Banned call scan: `0` matches for `Color32::from_rgb`, `Visuals::`, `unsafe`, `unwrap()`, `expect(`, `panic!`, `todo!`, `unreachable!`.
- Full cargo test suite: 1117 unit tests + 1 integration test passing green.
